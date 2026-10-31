//! Settings feature handler. Services retain system ownership and privilege checks.
use super::*;

impl Shell {
    pub(super) fn dispatch_settings(&self, surface: &Rc<WebSurface>, request: Request) -> Result<Reply, ErrorBody> {
        match request {
            Request::SettingsTime => Ok(Reply::Later(Box::pin(async move {
                let state = crate::files::blocking(crate::system_integrations::time).await?;
                proto::reply_ok(&state).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsTimeSet(change) => Ok(Reply::Later(Box::pin(async move {
                let state = crate::files::blocking(move || crate::system_integrations::set_time(change)).await?;
                proto::reply_ok(&state).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsPrinters => Ok(Reply::Later(Box::pin(async move {
                let state = crate::files::blocking(crate::system_integrations::printers).await?;
                proto::reply_ok(&state).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsInput => Ok(Reply::Later(Box::pin(async move {
                let value = crate::files::blocking(crate::input_preferences::get).await?;
                proto::reply_ok(&value).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsInputSet(value) => Ok(Reply::Later(Box::pin(async move {
                let result = crate::files::blocking(move || crate::input_preferences::set(value)).await?;
                proto::reply_ok(&result).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsSearch => serialize(proto::reply_ok(&*self.search_preferences.borrow())),
            Request::SettingsSearchSet(value) => {
                crate::search::save(&value)?;
                *self.search_preferences.borrow_mut() = value.clone();
                self.send_to_popover(Popover::Applications, &Event::SearchInvalidated);
                serialize(proto::reply_ok(&value))
            }
            Request::SettingsPrimaryDisplay | Request::SettingsPrimaryDisplaySet(_) => {
                if let Request::SettingsPrimaryDisplaySet(choice) = request {
                    if let Some(connector) = &choice.connector {
                        let display = gdk::Display::default()
                            .ok_or_else(|| ErrorBody::new(ErrorCode::Unavailable, "No display"))?;
                        let present = display
                            .monitors()
                            .iter::<gdk::Monitor>()
                            .filter_map(Result::ok)
                            .any(|monitor| monitor.connector().as_deref() == Some(connector.as_str()));
                        if connector.len() > 128 || !present {
                            return Err(ErrorBody::new(ErrorCode::InvalidRequest, "Choose a connected display"));
                        }
                    }
                    crate::desktop::set_primary_display(choice.connector)?;
                    self.reconcile_monitors();
                }
                let effective = self
                    .state
                    .borrow()
                    .primary
                    .as_ref()
                    .and_then(|monitor| monitor.connector())
                    .map(|name| name.to_string());
                serialize(proto::reply_ok(&proto::PrimaryDisplayState {
                    connector: crate::desktop::primary_display(),
                    effective,
                }))
            }

            Request::SettingsCapability(query) => Ok(Reply::Later(Box::pin(async move {
                let value = crate::capabilities::query(query.feature).await;
                proto::reply_ok(&value).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsStorage => Ok(Reply::Later(Box::pin(async move {
                let value = crate::files::blocking(crate::storage::list).await?;
                proto::reply_ok(&value).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsStorageAction(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::storage::perform(choice).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsDisplays => {
                // GDK's wl_output inventory remains useful on compositors without
                // wlr-output-management, including virtual and EDID-less outputs.
                let fallback: Vec<proto::DisplayOutput> = gdk::Display::default()
                    .map(|d| {
                        d.monitors()
                            .iter::<gdk::Monitor>()
                            .filter_map(Result::ok)
                            .map(|m| {
                                let geometry = m.geometry();
                                proto::DisplayOutput {
                                    name: m
                                        .connector()
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| "Virtual output".into()),
                                    description: m.model().map(|s| s.to_string()).unwrap_or_default(),
                                    enabled: true,
                                    x: geometry.x(),
                                    y: geometry.y(),
                                    scale: f64::from(m.scale_factor()),
                                    transform: 0,
                                    current: Some(proto::DisplayMode {
                                        width: geometry.width(),
                                        height: geometry.height(),
                                        refresh: m.refresh_rate(),
                                        preferred: false,
                                    }),
                                    modes: vec![],
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(Reply::Later(Box::pin(async move {
                    let outputs = crate::files::blocking(|| {
                        meridian_services::displays::list().map_err(|e| ErrorBody::new(ErrorCode::Unavailable, e))
                    })
                    .await
                    .unwrap_or(fallback);
                    proto::reply_ok(&outputs).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::SettingsDisplayApply(choice) => Ok(Reply::Later(Box::pin(async move {
                let trial =
                    crate::files::blocking(move || crate::settings_devices::display_apply(choice.outputs)).await?;
                proto::reply_ok(&trial).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsDisplayDecide(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || crate::settings_devices::display_decide(choice)).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsMonitorBrightness(choice) => Ok(Reply::Later(Box::pin(async move {
                let value = crate::files::blocking(move || crate::settings_devices::monitor_brightness(choice)).await?;
                proto::reply_ok(&value).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsWifiNetworks => Ok(Reply::Later(Box::pin(async move {
                let networks = crate::files::blocking(crate::settings_devices::wifi_networks).await?;
                proto::reply_ok(&networks).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsWifiScan => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(crate::settings_devices::wifi_scan).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsWifiProfiles => Ok(Reply::Later(Box::pin(async move {
                let profiles = crate::files::blocking(crate::settings_devices::wifi_profiles).await?;
                proto::reply_ok(&profiles).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsWifiConfig(choice) => Ok(Reply::Later(Box::pin(async move {
                let config = crate::files::blocking(move || crate::settings_devices::wifi_config(&choice.uuid)).await?;
                proto::reply_ok(&config).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsWifiConfigSet(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || crate::settings_devices::wifi_config_set(choice)).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsWifiProfileAction(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || crate::settings_devices::wifi_profile_action(choice)).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsWifiConnect(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || crate::settings_devices::wifi_connect(choice)).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsBluetoothDevices => Ok(Reply::Later(Box::pin(async move {
                let devices = crate::settings_devices::bluetooth_devices().await?;
                proto::reply_ok(&devices).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsBluetoothScan => Ok(Reply::Later(Box::pin(async move {
                crate::settings_devices::bluetooth_scan().await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsBluetoothAction(choice) => {
                let parent = Some(surface.window.clone());
                Ok(Reply::Later(Box::pin(async move {
                    crate::settings_devices::bluetooth_action(choice, parent).await?;
                    proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::SettingsBluetoothAudio => {
                let state = crate::audio_devices::bluetooth_audio();
                serialize(proto::reply_ok(&state))
            }
            Request::SettingsBluetoothAudioSet(choice) => Ok(Reply::Later(Box::pin(async move {
                let state = crate::files::blocking(move || crate::audio_devices::set_bluetooth_audio(choice)).await?;
                proto::reply_ok(&state).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsUsers => Ok(Reply::Later(Box::pin(async move {
                let users = crate::users::list().await?;
                proto::reply_ok(&users).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsUserCreate(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::users::create(choice).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsUserAction(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::users::change(choice).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsDefaultApps => serialize(proto::reply_ok(&crate::default_apps::list())),
            Request::SettingsDefaultAppSet(choice) => {
                let value = crate::default_apps::set(&choice, &self.catalog.borrow())?;
                serialize(proto::reply_ok(&value))
            }
            Request::SettingsStartupApps => {
                let apps: Vec<_> = self
                    .catalog
                    .borrow()
                    .apps()
                    .iter()
                    .filter(|a| a.source.is_some())
                    .map(|a| proto::SearchItem {
                        id: a.id.clone(),
                        title: a.name.clone(),
                        subtitle: a.description.clone(),
                        icon: a.icon.clone(),
                    })
                    .collect();
                serialize(proto::reply_ok(&apps))
            }
            Request::SettingsStartup => Ok(Reply::Later(Box::pin(async move {
                let entries = crate::files::blocking(crate::startup::list).await?;
                proto::reply_ok(&entries).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsStartupSet(choice) => Ok(Reply::Later(Box::pin(async move {
                let entries = crate::files::blocking(move || crate::startup::set(&choice.id, choice.enabled)).await?;
                proto::reply_ok(&entries).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsStartupAdd(choice) => {
                let source = self
                    .catalog
                    .borrow()
                    .get(&choice.id)
                    .and_then(|app| app.source.clone())
                    .ok_or_else(|| ErrorBody::new(ErrorCode::InvalidRequest, "Choose an installed application"))?;
                Ok(Reply::Later(Box::pin(async move {
                    let entries = crate::files::blocking(move || crate::startup::add(&choice.id, &source)).await?;
                    proto::reply_ok(&entries).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::SettingsSystemInfo => serialize(proto::reply_ok(&system_info())),
            Request::SettingsCheckUpdates => Ok(Reply::Later(Box::pin(async move {
                let update = crate::files::blocking(crate::updates::check).await?;
                proto::reply_ok(&update).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsOpenReleases => {
                gio::AppInfo::launch_default_for_uri(
                    "https://github.com/saikrishna488/meridian/releases",
                    None::<&gio::AppLaunchContext>,
                )
                .map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))?;
                serialize(ok())
            }
            Request::SettingsDevices => Ok(Reply::Later(Box::pin(async move {
                let devices = crate::files::blocking(crate::device_manager::list).await?;
                proto::reply_ok(&devices).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsDeviceAction(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || crate::device_manager::action(choice)).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsDrivers => Ok(Reply::Later(Box::pin(async move {
                let drivers = crate::files::blocking(crate::drivers::list).await?;
                proto::reply_ok(&drivers).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsDriverSet(choice) => Ok(Reply::Later(Box::pin(async move {
                crate::files::blocking(move || crate::drivers::change(choice)).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::PowerStatus => Ok(Reply::Later(Box::pin(async move {
                let power = crate::files::blocking(crate::power_settings::status).await?;
                proto::reply_ok(&power).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsIdle => serialize(proto::reply_ok(&crate::power_settings::idle())),
            Request::SettingsIdleSet(settings) => Ok(Reply::Later(Box::pin(async move {
                let settings = crate::files::blocking(move || crate::power_settings::save(settings)).await?;
                proto::reply_ok(&settings).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsAudio => Ok(Reply::Later(Box::pin(async move {
                let audio = crate::files::blocking(crate::audio_devices::list).await?;
                proto::reply_ok(&audio).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsAudioAction(choice) => {
                let settings = self.settings.clone();
                Ok(Reply::Later(Box::pin(async move {
                    let audio = crate::files::blocking(move || crate::audio_devices::action(choice)).await?;
                    settings.refresh();
                    proto::reply_ok(&audio).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
                })))
            }
            Request::SettingsGet => serialize(proto::reply_ok(&self.settings.state())),
            Request::SettingsSetWifi(v) => Ok(Reply::Later(Box::pin(async move {
                crate::settings_devices::wifi_enabled(v.enabled).await?;
                proto::reply_ok(&()).map_err(|e| ErrorBody::new(ErrorCode::Failed, e.to_string()))
            }))),
            Request::SettingsSetBluetooth(v) => {
                self.settings.set_bluetooth(v.enabled)?;
                serialize(ok())
            }
            Request::SettingsSetBrightness(v) => {
                self.settings.set_brightness(v.percent)?;
                serialize(ok())
            }
            Request::SettingsSetVolume(v) => {
                self.settings.set_volume(v.percent)?;
                serialize(ok())
            }
            _ => Err(ErrorBody::new(ErrorCode::InvalidRequest, "Not a settings request")),
        }
    }
}
