use super::*;
use crate::Error;
use tokio_util::sync::{CancellationToken, DropGuard};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::{runtime::Runtime};
use tokio::sync::{watch, mpsc};

#[derive(Debug, Clone)]
pub struct ConfigCtrl {
    cmds: mpsc::UnboundedSender<ConfigCmd>,
    state: watch::Receiver<ConfigState>,
    guard: Arc<DropGuard>,
}

impl ConfigCtrl {
    pub fn new(rt: &Runtime) -> Self {
        let ctok = CancellationToken::new();
        let guard = Arc::new(ctok.clone().drop_guard());
        let (state, state_) = watch::channel(ConfigState::Loading);
        let (cmds, cmds_) = mpsc::unbounded_channel();
        let _ = rt.spawn(Task::new(ctok, cmds_, state).run());
        Self { guard, cmds, state: state_ }
    }

    pub fn state(&mut self) -> &mut watch::Receiver<ConfigState> {
        &mut self.state
    }

    pub fn subscribe(&self) -> watch::Receiver<ConfigState> {
        self.state.clone()
    }

    pub fn set(&self, config: AppConfig) {
        let _ = self.cmds.send(ConfigCmd::Set(config));
    }

    pub fn reset(&self) {
        let _ = self.cmds.send(ConfigCmd::Reset);
    }

    pub fn reload(&self) {
        let _ = self.cmds.send(ConfigCmd::Reload);
    }
}

enum ConfigCmd {
    Set(AppConfig),
    Reset,
    Reload
}

#[derive(Debug, Clone)]
pub enum ConfigState {
    Loading,
    Result(Result<Option<AppConfig>, String>),
}

struct Task {
    ctok: CancellationToken,
    cmds: mpsc::UnboundedReceiver<ConfigCmd>,
    state: watch::Sender<ConfigState>,
}

impl Task {
    fn new(ctok: CancellationToken, cmds: mpsc::UnboundedReceiver<ConfigCmd>, state: watch::Sender<ConfigState>) -> Self {
        Self { ctok, cmds, state }
    }

    async fn run(mut self) {
        let r = self.load().await.map_err(|e| e.to_string());
        let _ = self.state.send(ConfigState::Result(r));

        loop {
            tokio::select! {
                _ = self.ctok.cancelled() => break,
                Some(cmd) = self.cmds.recv() => match cmd {
                    ConfigCmd::Set(config) => {
                        let r = self.store(&config).await.map(|_| Some(config)).map_err(|e| e.to_string());
                        let _ = self.state.send(ConfigState::Result(r));
                    },
                    ConfigCmd::Reset => {
                        let config = AppConfig::default();
                        let r = self.store(&config).await.map(|_| Some(config)).map_err(|e| e.to_string());
                        let _ = self.state.send(ConfigState::Result(r));
                    },
                    ConfigCmd::Reload => {
                        let r = self.load().await.map_err(|e| e.to_string());
                        let _ = self.state.send(ConfigState::Result(r));
                    },
                }
            }
        }
    }

    pub async fn load(&self) -> Result<Option<AppConfig>, Error> {
        let path = Self::find_dir().await?.join("config.toml");

        if path.exists() {
            let content = tokio::fs::read_to_string(path).await?;
            let config: AppConfig = toml::from_str(&content)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    pub async fn store(&self, config: &AppConfig) -> Result<(), Error> {
        let path = Self::find_dir().await?.join("config.toml");
        let content = toml::to_string_pretty(&config)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    pub async fn find_dir() -> Result<PathBuf, Error> {
        let dir1 = std::env::var("SHORELINE_CONFIG_DIR").ok().map(|x| PathBuf::from(x));
        let dir2 = std::env::current_dir().ok().map(|x| x.join(".shoreline"));
        let dir3 = std::env::home_dir().map(|x| x.join(".shoreline"));

        let dir = match (dir1, dir2, dir3) {
            (Some(p), _, _) => p,
            (_, Some(p), _) if p.exists() => p,
            (_, _, Some(p)) => p,
            _ => {
                return Err(
                    "Could not determine config path; set $SHORELINE_CONFIG_DIR or run from home directory".into()
                );
            }
        };

        tokio::fs::create_dir_all(&dir).await?;
        Ok(dir)
    }
}
