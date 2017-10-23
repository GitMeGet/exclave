use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::fmt;

use unitmanager::ManagerControlMessage;
use unit::{UnitKind, UnitName};

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum UnitStatus {
    /// A new unit file has appeared on the disk
    Added(PathBuf),

    /// A unit file on the disk has changed, and the unit will be reloaded
    Updated(PathBuf),

    /// We started to load the unit file
    LoadStarted(PathBuf /* path to the unit file that's gong away */),

    /// The unit file failed to load for some reason
    LoadFailed(String /* reason */),

    /// The unit file reported that it was not compatible
    Incompatible(String /* reason */),

    /// The unit has been selected, and may be made active later on.
    Selected,

    /// The unit has been deselected (but is still loaded, and may be selected later)
    Deselected,

    /// The unit is currently in use
    Active,

    /// We tried to activate, but failed to do so
    ActivationFailed(String /* reason */),

    /// The unit was active, then stopped being active due to finishing successfully
    DeactivatedSuccessfully(String /* reason */),

    /// The unit was active, then stopped being active due to finishing unsuccessfully
    DeactivatedUnsuccessfully(String /* reason */),

    /// The unit already successfully loaded, but is being removed
    UnloadStarted(PathBuf /* path to the unit file that's gong away */),

    /// The unit already successfully loaded, but is being updated
    UpdateStarted(PathBuf /* path to the unit file that's gong away */),

    /// The unit file was removed from the disk
    Removed(PathBuf),
}

impl fmt::Display for UnitStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &UnitStatus::Added(ref path) => write!(f, "Added {}", path.to_string_lossy()),
            &UnitStatus::Updated(ref path) => write!(f, "Updated {}", path.to_string_lossy()),
            &UnitStatus::LoadStarted(ref path) => write!(f, "Load started {}", path.to_string_lossy()),
            &UnitStatus::LoadFailed(ref x) => write!(f, "Load failed: {}", x),
            &UnitStatus::Incompatible(ref x) => write!(f, "Incompatible: {}", x),
            &UnitStatus::Selected => write!(f, "Selected"),
            &UnitStatus::Deselected => write!(f, "Deselected"),
            &UnitStatus::Active => write!(f, "Active"),
            &UnitStatus::ActivationFailed(ref reason) => write!(f, "Activation failed: {}", reason),
            &UnitStatus::DeactivatedSuccessfully(ref x) => {
                write!(f, "Deactivated successfully: {}", x)
            }
            &UnitStatus::DeactivatedUnsuccessfully(ref x) => {
                write!(f, "Deactivated unsuccessfilly: {}", x)
            }
            &UnitStatus::UnloadStarted(ref path) => write!(f, "Unloading {}", path.to_string_lossy()),
            &UnitStatus::UpdateStarted(ref path) => write!(f, "Updating {}", path.to_string_lossy()),
            &UnitStatus::Removed(ref path) => write!(f, "Removed {}", path.to_string_lossy()),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct UnitStatusEvent {
    pub name: UnitName,
    pub status: UnitStatus,
}

impl UnitStatusEvent {
    pub fn name(&self) -> &UnitName {
        &self.name
    }
    pub fn status(&self) -> &UnitStatus {
        &self.status
    }
    pub fn kind(&self) -> &UnitKind {
        &self.name.kind()
    }
    pub fn new_added(path: &Path) -> Option<UnitStatusEvent> {
        let name = match UnitName::from_path(path) {
            Err(_) => return None,
            Ok(s) => s,
        };

        Some(UnitStatusEvent {
            name: name,
            status: UnitStatus::Added(path.to_owned()),
        })
    }
    pub fn new_updated(path: &Path) -> Option<UnitStatusEvent> {
        let name = match UnitName::from_path(path) {
            Err(_) => return None,
            Ok(s) => s,
        };

        Some(UnitStatusEvent {
            name: name,
            status: UnitStatus::Updated(path.to_owned()),
        })
    }
    pub fn new_removed(path: &Path) -> Option<UnitStatusEvent> {
        let name = match UnitName::from_path(path) {
            Err(_) => return None,
            Ok(s) => s,
        };

        Some(UnitStatusEvent {
            name: name,
            status: UnitStatus::Removed(path.to_owned()),
        })
    }

    pub fn new_selected(name: &UnitName) -> UnitStatusEvent {
        UnitStatusEvent {
            name: name.clone(),
            status: UnitStatus::Selected,
        }
    }

    pub fn new_load_started(name: &UnitName, path: &PathBuf) -> UnitStatusEvent {
        UnitStatusEvent {
            name: name.clone(),
            status: UnitStatus::LoadStarted(path.clone()),
        }
    }

    pub fn new_update_started(name: &UnitName, path: &PathBuf) -> UnitStatusEvent {
        UnitStatusEvent {
            name: name.clone(),
            status: UnitStatus::UpdateStarted(path.clone()),
        }
    }

    pub fn new_load_failed(name: &UnitName, msg: String) -> UnitStatusEvent {
        UnitStatusEvent {
            name: name.clone(),
            status: UnitStatus::LoadFailed(msg),
        }
    }

    pub fn new_active(name: &UnitName) -> UnitStatusEvent {
        UnitStatusEvent {
            name: name.clone(),
            status: UnitStatus::Active,
        }
    }

    pub fn new_active_failed(name: &UnitName, msg: String) -> UnitStatusEvent {
        UnitStatusEvent {
            name: name.clone(),
            status: UnitStatus::ActivationFailed(msg),
        }
    }

    pub fn new_deactivate_success(name: &UnitName, msg: String) -> UnitStatusEvent {
        UnitStatusEvent {
            name: name.clone(),
            status: UnitStatus::DeactivatedSuccessfully(msg),
        }
    }

    pub fn new_deactivate_failure(name: &UnitName, msg: String) -> UnitStatusEvent {
        UnitStatusEvent {
            name: name.clone(),
            status: UnitStatus::DeactivatedUnsuccessfully(msg),
        }
    }

    pub fn new_unit_incompatible(name: &UnitName, msg: String) -> UnitStatusEvent {
        UnitStatusEvent {
            name: name.clone(),
            status: UnitStatus::Incompatible(msg),
        }
    }

    pub fn new_deselected(name: &UnitName) -> UnitStatusEvent {
        UnitStatusEvent {
            name: name.clone(),
            status: UnitStatus::Deselected,
        }
    }

    pub fn new_unload_started(name: &UnitName, path: &PathBuf) -> UnitStatusEvent {
        UnitStatusEvent {
            name: name.clone(),
            status: UnitStatus::UnloadStarted(path.clone()),
        }
    }
}

pub type UnitCategoryStatus = String;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct UnitCategoryEvent {
    kind: UnitKind,
    status: UnitCategoryStatus,
}

impl UnitCategoryEvent {
    pub fn new(kind: UnitKind, status: &UnitCategoryStatus) -> Self {
        UnitCategoryEvent {
            kind: kind,
            status: status.clone(),
        }
    }
    pub fn kind(&self) -> &UnitKind {
        &self.kind
    }
    pub fn status(&self) -> &UnitCategoryStatus {
        &self.status
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum UnitEvent {
    /// A unit has updated its status.
    Status(UnitStatusEvent),

    /// A whole category of units has been updated.
    Category(UnitCategoryEvent),

    /// The system has requested a rescan take place.
    RescanRequest,

    /// A rescan has started.
    RescanStart,

    /// The rescan has finished.
    RescanFinish,

    /// A unit made a request to a Manager, which will be passed to the main thread.
    ManagerRequest(ManagerControlMessage),

    /// The system is shutting down.
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct UnitBroadcaster {
    senders: Arc<Mutex<Vec<Sender<UnitEvent>>>>,
}

impl UnitBroadcaster {
    pub fn new() -> Self {
        UnitBroadcaster { senders: Arc::new(Mutex::new(vec![])) }
    }

    fn broadcast_core(senders: &Arc<Mutex<Vec<Sender<UnitEvent>>>>, event: &UnitEvent) {
        let mut to_remove = None;
        // Send a copy of the message to each of the listeners.
        let mut notify_senders_ref = senders.lock().unwrap();
        {
            for (idx, sender) in notify_senders_ref.iter().enumerate() {
                if let Err(e) = sender.send(event.clone()) {
                    eprintln!("Sender {} stopped responding: {:?}, removing it.", idx, e);
                    to_remove = Some(idx);
                }
            }
        }

        // If a sender threw an error, simply remove it from the list of elements to update
        if let Some(idx) = to_remove {
            notify_senders_ref.remove(idx);
        }
    }

    pub fn broadcast(&self, event: &UnitEvent) {
        Self::broadcast_core(&self.senders, event)
    }

    pub fn subscribe(&self) -> Receiver<UnitEvent> {
        let (sender, receiver) = channel();
        self.senders.lock().unwrap().push(sender);
        receiver
    }
}
