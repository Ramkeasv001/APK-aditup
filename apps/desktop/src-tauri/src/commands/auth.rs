use std::time::Instant;

use aditup_db::repositories::users::UsersRepository;
use aditup_db::{Database, DbError};
use aditup_security::AuthService;
use serde::Serialize;
use tauri::State;

use crate::state::{AppState, OWNER_USERNAME};

/// The encrypted database file *is* the authentication boundary for the
/// master password: opening it with the right passphrase is only possible
/// if the passphrase is correct (SQLCipher's own key check — see
/// `aditup_db::Database::open`). This command only reports whether that
/// file exists yet; it never opens it, since a wrong guess here shouldn't
/// count as a decryption attempt against the real file.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    pub needs_setup: bool,
}

#[tauri::command]
pub fn check_setup_status(state: State<AppState>) -> SetupStatus {
    SetupStatus {
        needs_setup: !state.db_path.exists(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupResult {
    /// Shown to the user exactly once — see `aditup_security::recovery`.
    pub recovery_key: String,
}

#[tauri::command]
pub fn run_setup(password: String, state: State<AppState>) -> Result<SetupResult, String> {
    if state.db_path.exists() {
        return Err("setup has already run".to_string());
    }
    if password.len() < 8 {
        return Err("password must be at least 8 characters".to_string());
    }

    // The file doesn't exist yet, so this creates it fresh, keyed with
    // `password` as the SQLCipher passphrase, and runs migrations.
    let db = Database::open(&state.db_path, &password).map_err(|e| e.to_string())?;
    let (_, recovery_key) = AuthService::new(&db)
        .setup(OWNER_USERNAME, &password)
        .map_err(|e| e.to_string())?;

    state.session.unlock(Instant::now());
    *state.db.lock().unwrap() = Some(db);

    Ok(SetupResult { recovery_key })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockResult {
    pub username: String,
}

#[tauri::command]
pub fn unlock(password: String, state: State<AppState>) -> Result<UnlockResult, String> {
    let now = Instant::now();
    state.unlock_limiter.check(now).map_err(|e| e.to_string())?;

    match Database::open(&state.db_path, &password) {
        Ok(db) => {
            // Successfully decrypting the file already proves the password
            // is correct; this lookup is for attribution (username/role),
            // not a second authentication check.
            let user = UsersRepository::new(&db)
                .get_by_username(OWNER_USERNAME)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "no account found in this database".to_string())?;

            state.unlock_limiter.record_success();
            state.session.unlock(now);
            *state.db.lock().unwrap() = Some(db);

            Ok(UnlockResult {
                username: user.username,
            })
        }
        Err(DbError::InvalidKey) => {
            state.unlock_limiter.record_failure(now);
            Err("incorrect password".to_string())
        }
        Err(other) => Err(other.to_string()),
    }
}

/// Drops the open `Database` (closing the file), marks the session locked,
/// and de-elevates Admin. Re-entering the password (`unlock`) is the only
/// way back in — there is no in-memory "just re-show the screen" shortcut,
/// since the key itself is gone.
#[tauri::command]
pub fn lock_session(state: State<AppState>) {
    state.lock();
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusDto {
    pub needs_setup: bool,
    pub is_locked: bool,
}

/// Polled by the frontend on mount (and could be polled periodically for
/// idle auto-lock) to decide whether to show Setup, Login, or the unlocked
/// app shell.
#[tauri::command]
pub fn session_status(state: State<AppState>) -> SessionStatusDto {
    SessionStatusDto {
        needs_setup: !state.db_path.exists(),
        is_locked: state.session.is_locked(Instant::now()),
    }
}
