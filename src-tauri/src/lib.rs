mod commands;
pub mod error;
pub mod files;
pub mod localfs;
pub mod nfs;
pub mod paths;
pub mod quit;
pub mod rclone;
pub mod remotes;
mod reveal;
pub mod settings;
pub mod text;

use files::transfers::{Item as TransferItem, TransferQueue};
use paths::AppPaths;
use rclone::{RcdState, Supervisor};
use remotes::health::HealthStore;
use reveal::{RevealGate, RevealState};
use settings::SettingsStore;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};

pub const RCD_STATE_EVENT: &str = "rcd://state";
pub const TRANSFERS_EVENT: &str = "transfers://update";

/// Closes the splash and shows the main window.
pub fn reveal_main_window(app: &AppHandle) {
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.unminimize();
        let _ = main.set_focus();
    }
}

fn on_rcd_change(app: &AppHandle, state: &RcdState) {
    let _ = app.emit(RCD_STATE_EVENT, state);
    if matches!(state, RcdState::Healthy { .. } | RcdState::Failed { .. }) {
        let reveal = app.state::<RevealState>();
        let ready = reveal.0.lock().expect("reveal lock").rclone_settled();
        if ready {
            reveal_main_window(app);
        }
    }
}

pub fn run() {
    let app = tauri::Builder::default()
        // Must come first: a second launch focuses the running Stingray and exits.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            reveal_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            app.manage(quit::QuitState(AtomicBool::new(false)));
            let data_dir = app.path().app_data_dir()?;
            let paths = AppPaths::new(&data_dir);
            std::fs::create_dir_all(paths.logs_dir())?;

            let mut settings = settings::load(&paths.settings()).unwrap_or_else(|err| {
                eprintln!("Stingray: could not read settings, using defaults: {err}");
                settings::Settings::default()
            });
            let spec = rclone::spec_for(&paths, &settings);
            let app_version = app.package_info().version.to_string();
            let splash_plan = settings.splash_plan(&app_version);
            if splash_plan == settings::SplashPlan::Full {
                // Remember that this version has shown its full intro, so the next launch is short.
                settings.last_splash_version = Some(app_version);
                if let Err(err) = settings::save(&paths.settings(), &settings) {
                    eprintln!("Stingray: could not save settings: {err}");
                }
            }
            if splash_plan == settings::SplashPlan::Off
                && let Some(splash) = app.get_webview_window("splash")
            {
                let _ = splash.close();
            }
            app.manage(splash_plan);
            app.manage(SettingsStore::new(settings.clone()));
            app.manage(RevealState(Mutex::new(RevealGate::new(
                splash_plan != settings::SplashPlan::Off,
            ))));
            app.manage(HealthStore::default());
            app.manage(nfs::NfsStore::load(paths.nfs_shares()));

            let handle = app.handle().clone();
            let supervisor = Supervisor::start(spec, move |state| on_rcd_change(&handle, state));
            let transfer_handle = app.handle().clone();
            let queue = TransferQueue::start(
                supervisor.clone(),
                settings.transfer_concurrency as usize,
                move |items: &[TransferItem]| {
                    let _ = transfer_handle.emit(TRANSFERS_EVENT, items);
                },
            );
            app.manage(queue);
            app.manage(supervisor);
            app.manage(paths);
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Closing the main window quits, but asks first when transfers are running.
                api.prevent_close();
                quit::request(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_rcd_state,
            commands::restart_rcd,
            commands::splash_finished,
            commands::get_splash_plan,
            commands::app_info,
            commands::get_settings,
            commands::save_settings,
            remotes::commands::list_providers,
            remotes::commands::list_remotes,
            remotes::commands::get_remote_form,
            remotes::commands::create_remote,
            remotes::commands::start_browser_sign_in,
            remotes::commands::cancel_browser_sign_in,
            remotes::commands::update_remote,
            remotes::commands::rename_remote,
            remotes::commands::delete_remote,
            remotes::commands::test_remote,
            remotes::commands::account_info,
            remotes::commands::reconnect_remote,
            remotes::commands::onedrive_libraries,
            remotes::commands::set_onedrive_library,
            files::commands::inspect_local_paths,
            files::commands::list_files,
            files::commands::create_folder,
            files::commands::delete_items,
            files::commands::rename_item,
            files::commands::enqueue_transfers,
            files::commands::list_transfers,
            files::commands::cancel_transfer,
            files::commands::clear_finished_transfers,
            localfs::home_places,
            localfs::list_local,
            localfs::local_create_folder,
            localfs::local_rename,
            localfs::local_trash,
            localfs::open_local,
            nfs::nfs_support,
            nfs::list_nfs_shares,
            nfs::add_nfs_share,
            nfs::connect_nfs_share,
            nfs::disconnect_nfs_share,
            nfs::remove_nfs_share,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Stingray");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            let already = app_handle
                .try_state::<quit::QuitState>()
                .is_some_and(|state| state.0.load(Ordering::SeqCst));
            if !already {
                let handle = app_handle.clone();
                tauri::async_runtime::block_on(async move {
                    let _ = tokio::time::timeout(quit::WATCHDOG, quit::shutdown_services(&handle))
                        .await;
                });
            }
        }
    });
}
