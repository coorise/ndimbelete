//! Typed Tauri `invoke` wrappers.
//!
//! Argument names match Rust command parameter names exactly
//! (e.g. `create_staff` expects `{ input: CreateStaffInput }`).

use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use super::types::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = "invoke")]
    fn invoke_js(cmd: &str, args: JsValue) -> js_sys::Promise;
}

fn err_to_string(err: JsValue) -> String {
    if let Some(s) = err.as_string() {
        return s;
    }
    js_sys::JSON::stringify(&err)
        .ok()
        .and_then(|s| s.as_string())
        .unwrap_or_else(|| "Erreur inconnue".into())
}

/// Low-level invoke that maps rejected promises to `Err(String)`.
pub async fn invoke<T: DeserializeOwned>(cmd: &str, args: impl Serialize) -> Result<T, String> {
    let args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    match JsFuture::from(invoke_js(cmd, args)).await {
        Ok(val) => {
            // `()` / null success payloads
            if val.is_undefined() || val.is_null() {
                return serde_wasm_bindgen::from_value(JsValue::NULL)
                    .or_else(|_| serde_wasm_bindgen::from_value(JsValue::from(js_sys::Object::new())))
                    .map_err(|e| e.to_string());
            }
            serde_wasm_bindgen::from_value(val).map_err(|e| e.to_string())
        }
        Err(err) => Err(err_to_string(err)),
    }
}

async fn invoke_unit(cmd: &str, args: impl Serialize) -> Result<(), String> {
    let args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    match JsFuture::from(invoke_js(cmd, args)).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err_to_string(err)),
    }
}

#[derive(Serialize)]
struct Empty {}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InputArg<T: Serialize> {
    input: T,
}

#[derive(Serialize)]
struct IdArg<'a> {
    id: &'a str,
}

#[derive(Serialize)]
struct IdsArg {
    ids: Vec<String>,
}

#[derive(Serialize)]
struct YearArg {
    year: i32,
}

#[derive(Serialize)]
struct QueryArg<'a> {
    query: &'a str,
}

#[derive(Serialize)]
struct SettingsArg<'a> {
    settings: &'a AppSettings,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MonthlyAmountArg {
    year: i32,
    amount: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordPaymentArg<'a> {
    member_id: &'a str,
    period_id: &'a str,
    amount: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewExcelArg<'a> {
    path: &'a str,
    sheet_name: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportExcelArg<'a> {
    path: &'a str,
    sheet_name: &'a str,
    year: i32,
    role_values: Option<&'a [String]>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportExcelGridArg<'a> {
    headers: &'a [String],
    rows: &'a [Vec<String>],
    year: i32,
    sheet_label: Option<&'a str>,
    role_values: Option<&'a [String]>,
    prune_missing: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportExcelArg<'a> {
    path: &'a str,
    year: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClearPaymentArg<'a> {
    member_id: &'a str,
    period_id: &'a str,
}

#[derive(Serialize)]
struct BackupPathArg<'a> {
    path: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OverviewStatsArg<'a> {
    year: i32,
    member_id: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MemberDebtAsOfArg<'a> {
    member_id: &'a str,
    year: i32,
    as_of_month: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DefaultNameArg {
    default_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeletePlanningArg<'a> {
    id: &'a str,
    force: Option<bool>,
}

// —— Auth ——

pub async fn needs_setup() -> Result<bool, String> {
    invoke("needs_setup", Empty {}).await
}

pub async fn setup_admin(
    first_name: String,
    last_name: String,
    username: String,
    password: String,
    phone: String,
    recovery_school: String,
    recovery_color: String,
) -> Result<SessionInfo, String> {
    invoke(
        "setup_admin",
        InputArg {
            input: SetupAdminInput {
                first_name,
                last_name,
                username,
                password,
                phone: if phone.trim().is_empty() {
                    None
                } else {
                    Some(phone)
                },
                recovery_school: if recovery_school.trim().is_empty() {
                    None
                } else {
                    Some(recovery_school)
                },
                recovery_color: if recovery_color.trim().is_empty() {
                    None
                } else {
                    Some(recovery_color)
                },
            },
        },
    )
    .await
}

pub async fn login(username: String, password: String) -> Result<SessionInfo, String> {
    invoke(
        "login",
        InputArg {
            input: LoginInput { username, password },
        },
    )
    .await
}

pub async fn logout() -> Result<(), String> {
    invoke_unit("logout", Empty {}).await
}

pub async fn get_current_session() -> Result<Option<SessionInfo>, String> {
    invoke("get_current_session", Empty {}).await
}

pub async fn change_password(
    current_password: String,
    new_password: String,
) -> Result<(), String> {
    invoke_unit(
        "change_password",
        InputArg {
            input: ChangePasswordInput {
                current_password,
                new_password,
            },
        },
    )
    .await
}

#[derive(Serialize)]
struct UsernameArg {
    username: String,
}

pub async fn get_recovery_status(username: String) -> Result<RecoveryStatus, String> {
    invoke("get_recovery_status", UsernameArg { username }).await
}

pub async fn reset_password_with_hints(
    input: ResetPasswordWithHintsInput,
) -> Result<(), String> {
    invoke_unit("reset_password_with_hints", InputArg { input }).await
}

pub async fn update_my_profile(input: UpdateMyProfileInput) -> Result<Staff, String> {
    invoke("update_my_profile", InputArg { input }).await
}

// —— Staff ——

pub async fn list_staff() -> Result<Vec<Staff>, String> {
    invoke("list_staff", Empty {}).await
}

pub async fn create_staff(input: CreateStaffInput) -> Result<Staff, String> {
    invoke("create_staff", InputArg { input }).await
}

pub async fn update_staff(input: UpdateStaffInput) -> Result<Staff, String> {
    invoke("update_staff", InputArg { input }).await
}

pub async fn deactivate_staff(id: &str) -> Result<(), String> {
    invoke_unit("deactivate_staff", IdArg { id }).await
}

// —— Roles ——

pub async fn list_roles() -> Result<Vec<Role>, String> {
    invoke("list_roles", Empty {}).await
}

pub async fn create_role(input: CreateRoleInput) -> Result<Role, String> {
    invoke("create_role", InputArg { input }).await
}

pub async fn update_role(input: UpdateRoleInput) -> Result<Role, String> {
    invoke("update_role", InputArg { input }).await
}

// —— Member roles ——

pub async fn list_member_roles() -> Result<Vec<MemberRole>, String> {
    invoke("list_member_roles", Empty {}).await
}

pub async fn create_member_role(input: CreateMemberRoleInput) -> Result<MemberRole, String> {
    invoke("create_member_role", InputArg { input }).await
}

pub async fn update_member_role(input: UpdateMemberRoleInput) -> Result<MemberRole, String> {
    invoke("update_member_role", InputArg { input }).await
}

// —— Members ——

pub async fn list_members() -> Result<Vec<Member>, String> {
    invoke("list_members", Empty {}).await
}

pub async fn create_member(input: CreateMemberInput) -> Result<Member, String> {
    invoke("create_member", InputArg { input }).await
}

pub async fn update_member(input: UpdateMemberInput) -> Result<Member, String> {
    invoke("update_member", InputArg { input }).await
}

pub async fn search_members(query: &str) -> Result<Vec<Member>, String> {
    invoke("search_members", QueryArg { query }).await
}

pub async fn delete_members(ids: Vec<String>) -> Result<usize, String> {
    invoke("delete_members", IdsArg { ids }).await
}

// —— Cotisations ——

pub async fn ensure_year(year: i32) -> Result<ContributionYear, String> {
    invoke("ensure_year", YearArg { year }).await
}

pub async fn get_year_grid(year: i32) -> Result<YearGrid, String> {
    invoke("get_year_grid", YearArg { year }).await
}

pub async fn set_monthly_amount(year: i32, amount: f64) -> Result<ContributionYear, String> {
    invoke("set_monthly_amount", MonthlyAmountArg { year, amount }).await
}

pub async fn record_payment(
    member_id: &str,
    period_id: &str,
    amount: f64,
) -> Result<MemberDebtSummary, String> {
    invoke(
        "record_payment",
        RecordPaymentArg {
            member_id,
            period_id,
            amount,
        },
    )
    .await
}

pub async fn get_member_debt_as_of(
    member_id: &str,
    year: i32,
    as_of_month: i32,
) -> Result<MemberDebtSummary, String> {
    invoke(
        "get_member_debt_as_of",
        MemberDebtAsOfArg {
            member_id,
            year,
            as_of_month,
        },
    )
    .await
}

pub async fn clear_payment(
    member_id: &str,
    period_id: &str,
) -> Result<MemberDebtSummary, String> {
    invoke(
        "clear_payment",
        ClearPaymentArg {
            member_id,
            period_id,
        },
    )
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetPeriodCellArg<'a> {
    member_id: &'a str,
    period_id: &'a str,
    field: &'a str,
    value: Option<f64>,
}

pub async fn set_period_cell(
    member_id: &str,
    period_id: &str,
    field: &str,
    value: Option<f64>,
) -> Result<MemberDebtSummary, String> {
    invoke(
        "set_period_cell",
        SetPeriodCellArg {
            member_id,
            period_id,
            field,
            value,
        },
    )
    .await
}

pub async fn recalculate_all_members(year: i32) -> Result<usize, String> {
    invoke("recalculate_all_members", YearArg { year }).await
}

// —— Planning ——

pub async fn list_planning(year: i32) -> Result<Vec<PlanningPeriod>, String> {
    invoke("list_planning", YearArg { year }).await
}

pub async fn upsert_planning_period(input: UpsertPlanningInput) -> Result<PlanningPeriod, String> {
    invoke("upsert_planning_period", InputArg { input }).await
}

pub async fn delete_planning_period(id: &str, force: bool) -> Result<(), String> {
    invoke_unit(
        "delete_planning_period",
        DeletePlanningArg {
            id,
            force: Some(force),
        },
    )
    .await
}

// —— Settings ——

pub async fn get_settings() -> Result<AppSettings, String> {
    invoke("get_settings", Empty {}).await
}

pub async fn update_settings(settings: &AppSettings) -> Result<AppSettings, String> {
    invoke("update_settings", SettingsArg { settings }).await
}

// —— Analytics ——

pub async fn get_overview_stats(
    year: i32,
    member_id: Option<&str>,
) -> Result<OverviewStats, String> {
    invoke(
        "get_overview_stats",
        OverviewStatsArg { year, member_id },
    )
    .await
}

// —— Excel ——

pub async fn preview_excel(
    path: &str,
    sheet_name: Option<&str>,
) -> Result<ExcelPreview, String> {
    invoke(
        "preview_excel",
        PreviewExcelArg { path, sheet_name },
    )
    .await
}

pub async fn import_excel(
    path: &str,
    sheet_name: &str,
    year: i32,
    role_values: Option<&[String]>,
) -> Result<ImportResult, String> {
    invoke(
        "import_excel",
        ImportExcelArg {
            path,
            sheet_name,
            year,
            role_values,
        },
    )
    .await
}

pub async fn import_excel_grid(
    headers: &[String],
    rows: &[Vec<String>],
    year: i32,
    sheet_label: Option<&str>,
    role_values: Option<&[String]>,
    prune_missing: bool,
) -> Result<ImportResult, String> {
    invoke(
        "import_excel_grid",
        ImportExcelGridArg {
            headers,
            rows,
            year,
            sheet_label,
            role_values,
            prune_missing: Some(prune_missing),
        },
    )
    .await
}

pub async fn export_excel(path: &str, year: i32) -> Result<ExportResult, String> {
    invoke("export_excel", ExportExcelArg { path, year }).await
}

// —— Backup ——

pub async fn export_backup(path: &str) -> Result<(), String> {
    invoke_unit("export_backup", BackupPathArg { path }).await
}

pub async fn import_backup(path: &str) -> Result<(), String> {
    invoke_unit("import_backup", BackupPathArg { path }).await
}

pub async fn restart_app() -> Result<(), String> {
    invoke_unit("restart_app", Empty {}).await
}

/// Native Excel file picker (Rust dialog plugin).
pub async fn pick_excel_file() -> Option<String> {
    invoke::<Option<String>>("pick_excel_file", Empty {})
        .await
        .ok()
        .flatten()
}

/// Native backup file picker.
pub async fn pick_backup_file() -> Option<String> {
    invoke::<Option<String>>("pick_backup_file", Empty {})
        .await
        .ok()
        .flatten()
}

/// Native save-as dialog.
pub async fn pick_save_file(default_name: &str) -> Option<String> {
    invoke::<Option<String>>(
        "pick_save_file",
        DefaultNameArg {
            default_name: default_name.to_string(),
        },
    )
    .await
    .ok()
    .flatten()
}

// —— Desktop print ——

pub async fn list_printers() -> Result<Vec<PrinterInfo>, String> {
    invoke("list_printers", Empty {}).await
}

pub async fn print_receipt(input: &PrintReceiptInput) -> Result<PrintReceiptResult, String> {
    #[derive(Serialize)]
    struct Arg<'a> {
        input: &'a PrintReceiptInput,
    }
    invoke("print_receipt", Arg { input }).await
}

pub async fn export_receipt_pdf(
    receipt: &ReceiptPrintPayload,
    path: &str,
) -> Result<PrintReceiptResult, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Arg<'a> {
        receipt: &'a ReceiptPrintPayload,
        path: &'a str,
    }
    invoke("export_receipt_pdf", Arg { receipt, path }).await
}

pub async fn get_app_version_info() -> Result<AppVersionInfo, String> {
    invoke("get_app_version_info", Empty {}).await
}

pub async fn check_for_update() -> Result<UpdateInfo, String> {
    invoke("check_for_update", Empty {}).await
}

pub async fn download_and_install_update() -> Result<(), String> {
    invoke_unit("download_and_install_update", Empty {}).await
}

// —— Collaboration ——

pub async fn collab_get_default_uri() -> Result<Option<String>, String> {
    invoke("collab_get_default_uri", Empty {}).await
}

pub async fn collab_probe(uri: &str) -> Result<RemoteProbe, String> {
    #[derive(Serialize)]
    struct Arg<'a> {
        uri: &'a str,
    }
    invoke("collab_probe", Arg { uri }).await
}

pub async fn collab_status() -> Result<CollabStatus, String> {
    invoke("collab_status", Empty {}).await
}

pub async fn collab_connect(
    uri: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<CollabConnectResult, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Arg<'a> {
        uri: &'a str,
        username: Option<&'a str>,
        password: Option<&'a str>,
    }
    invoke(
        "collab_connect",
        Arg {
            uri,
            username,
            password,
        },
    )
    .await
}

pub async fn collab_disconnect() -> Result<(), String> {
    invoke_unit("collab_disconnect", Empty {}).await
}

pub async fn collab_push(message: Option<&str>) -> Result<CollabPushResult, String> {
    #[derive(Serialize)]
    struct Arg {
        message: Option<String>,
    }
    invoke(
        "collab_push",
        Arg {
            message: message.map(|s| s.to_string()),
        },
    )
    .await
}

pub async fn collab_pull() -> Result<CollabStatus, String> {
    invoke("collab_pull", Empty {}).await
}

pub async fn collab_list_commits() -> Result<Vec<CollabCommitInfo>, String> {
    invoke("collab_list_commits", Empty {}).await
}

pub async fn collab_list_activities(limit: Option<i64>) -> Result<Vec<ActivityEntry>, String> {
    #[derive(Serialize)]
    struct Arg {
        limit: Option<i64>,
    }
    invoke("collab_list_activities", Arg { limit }).await
}

pub async fn collab_rollback(commit_id: &str) -> Result<CollabStatus, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Arg<'a> {
        commit_id: &'a str,
    }
    invoke("collab_rollback", Arg { commit_id }).await
}

pub async fn collab_cleanup(keep: Option<u32>) -> Result<u64, String> {
    #[derive(Serialize)]
    struct Arg {
        keep: Option<u32>,
    }
    invoke("collab_cleanup", Arg { keep }).await
}

pub async fn collab_set_keep_commits(keep: u32) -> Result<(), String> {
    #[derive(Serialize)]
    struct Arg {
        keep: u32,
    }
    invoke_unit("collab_set_keep_commits", Arg { keep }).await
}

pub async fn collab_set_push_acl(
    role_ids: Vec<String>,
    staff_ids: Vec<String>,
) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Arg {
        role_ids: Vec<String>,
        staff_ids: Vec<String>,
    }
    invoke_unit("collab_set_push_acl", Arg { role_ids, staff_ids }).await
}

pub async fn collab_clear_remote() -> Result<(), String> {
    invoke_unit("collab_clear_remote", Empty {}).await
}
