use std::time::Instant;

use aditup_security::AuthService;
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub fn is_admin_pin_set(state: State<AppState>) -> Result<bool, String> {
    state.with_db(|db| {
        AuthService::new(db)
            .is_admin_pin_set()
            .map_err(|e| e.to_string())
    })
}

/// First-time Admin PIN setup. Elevates immediately on success — the Admin
/// who just set the PIN shouldn't have to re-enter it to prove they know
/// it.
#[tauri::command]
pub fn set_admin_pin(pin: String, state: State<AppState>) -> Result<(), String> {
    state.with_db(|db| {
        AuthService::new(db)
            .set_admin_pin(&pin)
            .map_err(|e| e.to_string())
    })?;
    *state.admin_elevated.lock().unwrap() = true;
    Ok(())
}

/// The Admin Console elevation gate (§4/§9): independent of the login
/// password, subject to its own 3-attempt lockout (`admin_pin_limiter` in
/// `AppState`, not `AuthService`'s own per-call limiter — see the comment
/// there for why).
#[tauri::command]
pub fn verify_admin_pin(pin: String, state: State<AppState>) -> Result<(), String> {
    let now = Instant::now();
    state
        .admin_pin_limiter
        .check(now)
        .map_err(|e| e.to_string())?;

    let result = state.with_db(|db| {
        AuthService::new(db)
            .verify_admin_pin(&pin, now)
            .map_err(|e| e.to_string())
    });

    match result {
        Ok(()) => {
            state.admin_pin_limiter.record_success();
            *state.admin_elevated.lock().unwrap() = true;
            Ok(())
        }
        Err(message) => {
            state.admin_pin_limiter.record_failure(now);
            Err(message)
        }
    }
}

/// "Exit Admin Console" — de-elevates without locking the whole app.
#[tauri::command]
pub fn exit_admin(state: State<AppState>) {
    *state.admin_elevated.lock().unwrap() = false;
}
