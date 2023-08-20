// File taken from https://github.com/servicer-labs/servicer/blob/master/src/utils/systemd.rs

use anyhow::Result;
use tracing::info;
use zbus::Connection;
use zbus::{dbus_proxy, zvariant};

#[derive(Debug, Clone)]
pub struct UnitStatus {
  pub name: String,         // The primary unit name as string
  pub description: String,  // The human readable description string
  pub load_state: String,   // The load state (i.e. whether the unit file has been loaded successfully)
  pub active_state: String, // The active state (i.e. whether the unit is currently started or not)
  pub sub_state: String, // The sub state (a more fine-grained version of the active state that is specific to the unit type, which the active state is not)
  pub followed: String, // A unit that is being followed in its state by this unit, if there is any, otherwise the empty string.
  pub path: String,     // The unit object path
  pub job_id: u32,      // If there is a job queued for the job unit the numeric job id, 0 otherwise
  pub job_type: String, // The job type as string
  pub job_path: String, // The job object path
}

type RawUnit =
  (String, String, String, String, String, String, zvariant::OwnedObjectPath, u32, String, zvariant::OwnedObjectPath);

impl From<RawUnit> for UnitStatus {
  fn from(raw_unit: RawUnit) -> Self {
    let (name, description, load_state, active_state, sub_state, followed, path, job_id, job_type, job_path) = raw_unit;

    Self {
      name,
      description,
      load_state,
      active_state,
      sub_state,
      followed,
      path: path.to_string(),
      job_id,
      job_type,
      job_path: job_path.to_string(),
    }
  }
}

// this takes like 5-10 ms on 13th gen Intel i7
pub async fn get_services() -> Result<Vec<UnitStatus>> {
  let start = std::time::Instant::now();
  let connection = Connection::system().await?;
  let manager_proxy = ManagerProxy::new(&connection).await?;
  let units = manager_proxy.list_units_by_patterns(vec![], vec!["*.service".into()]).await?;

  let mut units: Vec<_> = units.into_iter().map(|u| UnitStatus::from(u)).collect();

  // sort by name case-insensitive
  units.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

  info!("Loaded systemd services in {:?}", start.elapsed());

  Ok(units)
}

/// Proxy object for `org.freedesktop.systemd1.Manager`.
/// Taken from https://github.com/lucab/zbus_systemd/blob/main/src/systemd1/generated.rs
#[dbus_proxy(
  interface = "org.freedesktop.systemd1.Manager",
  default_service = "org.freedesktop.systemd1",
  default_path = "/org/freedesktop/systemd1",
  gen_blocking = false
)]
pub trait Manager {
  /// [📖](https://www.freedesktop.org/software/systemd/man/systemd.directives.html#StartUnit()) Call interface method `StartUnit`.
  #[dbus_proxy(name = "StartUnit")]
  fn start_unit(&self, name: String, mode: String) -> zbus::Result<zvariant::OwnedObjectPath>;

  /// [📖](https://www.freedesktop.org/software/systemd/man/systemd.directives.html#StopUnit()) Call interface method `StopUnit`.
  #[dbus_proxy(name = "StopUnit")]
  fn stop_unit(&self, name: String, mode: String) -> zbus::Result<zvariant::OwnedObjectPath>;

  /// [📖](https://www.freedesktop.org/software/systemd/man/systemd.directives.html#EnableUnitFiles()) Call interface method `EnableUnitFiles`.
  #[dbus_proxy(name = "EnableUnitFiles")]
  fn enable_unit_files(
    &self,
    files: Vec<String>,
    runtime: bool,
    force: bool,
  ) -> zbus::Result<(bool, Vec<(String, String, String)>)>;

  /// [📖](https://www.freedesktop.org/software/systemd/man/systemd.directives.html#DisableUnitFiles()) Call interface method `DisableUnitFiles`.
  #[dbus_proxy(name = "DisableUnitFiles")]
  fn disable_unit_files(&self, files: Vec<String>, runtime: bool) -> zbus::Result<Vec<(String, String, String)>>;

  /// [📖](https://www.freedesktop.org/software/systemd/man/systemd.directives.html#ListUnits()) Call interface method `ListUnits`.
  #[dbus_proxy(name = "ListUnits")]
  fn list_units(
    &self,
  ) -> zbus::Result<
    Vec<(
      String,
      String,
      String,
      String,
      String,
      String,
      zvariant::OwnedObjectPath,
      u32,
      String,
      zvariant::OwnedObjectPath,
    )>,
  >;

  /// [📖](https://www.freedesktop.org/software/systemd/man/systemd.directives.html#ListUnitsByPatterns()) Call interface method `ListUnitsByPatterns`.
  #[dbus_proxy(name = "ListUnitsByPatterns")]
  fn list_units_by_patterns(
    &self,
    states: Vec<String>,
    patterns: Vec<String>,
  ) -> zbus::Result<
    Vec<(
      String,
      String,
      String,
      String,
      String,
      String,
      zvariant::OwnedObjectPath,
      u32,
      String,
      zvariant::OwnedObjectPath,
    )>,
  >;

  /// [📖](https://www.freedesktop.org/software/systemd/man/systemd.directives.html#Reload()) Call interface method `Reload`.
  #[dbus_proxy(name = "Reload")]
  fn reload(&self) -> zbus::Result<()>;
}

/// Proxy object for `org.freedesktop.systemd1.Unit`.
/// Taken from https://github.com/lucab/zbus_systemd/blob/main/src/systemd1/generated.rs
#[dbus_proxy(
  interface = "org.freedesktop.systemd1.Unit",
  default_service = "org.freedesktop.systemd1",
  assume_defaults = false,
  gen_blocking = false
)]
pub trait Unit {
  /// Get property `ActiveState`.
  #[dbus_proxy(property)]
  fn active_state(&self) -> zbus::Result<String>;

  /// Get property `LoadState`.
  #[dbus_proxy(property)]
  fn load_state(&self) -> zbus::Result<String>;

  /// Get property `UnitFileState`.
  #[dbus_proxy(property)]
  fn unit_file_state(&self) -> zbus::Result<String>;
}

/// Proxy object for `org.freedesktop.systemd1.Service`.
/// Taken from https://github.com/lucab/zbus_systemd/blob/main/src/systemd1/generated.rs
#[dbus_proxy(
  interface = "org.freedesktop.systemd1.Service",
  default_service = "org.freedesktop.systemd1",
  assume_defaults = false,
  gen_blocking = false
)]
trait Service {
  /// Get property `MainPID`.
  #[dbus_proxy(property, name = "MainPID")]
  fn main_pid(&self) -> zbus::Result<u32>;
}

/// Returns the load state of a systemd unit
///
/// Returns `invalid-unit-path` if the path is invalid
///
/// # Arguments
///
/// * `connection`: zbus connection
/// * `full_service_name`: Full name of the service name with '.service' in the end
///
pub async fn get_active_state(connection: &Connection, full_service_name: &String) -> String {
  let object_path = get_unit_path(full_service_name);

  match zvariant::ObjectPath::try_from(object_path) {
    Ok(path) => {
      let unit_proxy = UnitProxy::new(connection, path).await.unwrap();
      unit_proxy.active_state().await.unwrap_or("invalid-unit-path".into())
    },
    Err(_) => "invalid-unit-path".to_string(),
  }
}

/// Returns the unit file state of a systemd unit. If the state is `enabled`, the unit loads on every boot
///
/// Returns `invalid-unit-path` if the path is invalid
///
/// # Arguments
///
/// * `connection`: zbus connection
/// * `full_service_name`: Full name of the service name with '.service' in the end
///
pub async fn get_unit_file_state(connection: &Connection, full_service_name: &String) -> String {
  let object_path = get_unit_path(full_service_name);

  match zvariant::ObjectPath::try_from(object_path) {
    Ok(path) => {
      let unit_proxy = UnitProxy::new(connection, path).await.unwrap();
      unit_proxy.unit_file_state().await.unwrap_or("invalid-unit-path".into())
    },
    Err(_) => "invalid-unit-path".to_string(),
  }
}

/// Returns the PID of a systemd service
///
/// # Arguments
///
/// * `connection`: zbus connection
/// * `full_service_name`: Full name of the service name with '.service' in the end
///
pub async fn get_main_pid(connection: &Connection, full_service_name: &String) -> Result<u32, zbus::Error> {
  let object_path = get_unit_path(full_service_name);

  let validated_object_path = zvariant::ObjectPath::try_from(object_path).unwrap();

  let service_proxy = ServiceProxy::new(connection, validated_object_path).await.unwrap();
  service_proxy.main_pid().await
}

/// Encode into a valid dbus string
///
/// # Arguments
///
/// * `input_string`
///
fn encode_as_dbus_object_path(input_string: &str) -> String {
  input_string
    .chars()
    .map(|c| if c.is_ascii_alphanumeric() || c == '/' || c == '_' { c.to_string() } else { format!("_{:x}", c as u32) })
    .collect()
}

/// Unit file path for a service
///
/// # Arguments
///
/// * `full_service_name`
///
pub fn get_unit_path(full_service_name: &str) -> String {
  format!("/org/freedesktop/systemd1/unit/{}", encode_as_dbus_object_path(full_service_name))
}