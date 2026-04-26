use crate::PathAddr;
use crate::path::PathStats;
use std::collections::BTreeMap;
use tokio::sync::watch;

#[derive(Debug, Clone)]
pub struct MultiPathStats {
    pub paths: BTreeMap<PathAddr, watch::Receiver<PathStats>>,
}
