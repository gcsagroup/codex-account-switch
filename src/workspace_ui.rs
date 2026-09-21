//! 账户搜索及备份交互。预览持有已经校验的数据，确认时不重新读取外部文件。
use crate::{
    backup, i18n::Language, paths, refresh_ui, AccountRow, AppShared, AppTray, MainWindow,
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::sync::{Arc, Mutex};

enum Action {
    Export,
    Import(backup::ImportPlan),
}

fn confirm_label(lang: Language, export: bool) -> &'static str {
    let (save, import) = match lang {
        Language::ZhCn => ("选择保存位置", "确认导入"),
        Language::ZhTw => ("選擇儲存位置", "確認匯入"),
        Language::En => ("Choose save location", "Confirm import"),
        Language::Ja => ("保存先を選択", "読み込みを確認"),
        Language::Ko => ("저장 위치 선택", "가져오기 확인"),
        Language::Fr => ("Choisir un emplacement", "Confirmer l’import"),
        Language::Es => ("Elegir ubicación", "Confirmar importación"),
    };
    if export {
        save
    } else {
        import
    }
}

pub fn labels(lang: Language) -> [&'static str; 16] {
    match lang {
        Language::ZhCn => ["搜索账户名、邮箱或套餐", "导入备份", "导出全部", "全部账户备份", "包含 Codex 与 Claude Desktop 已保存账户的登录凭据。文件未加密，请保存在可信的本地位置，不要分享或上传。\n不包含未保存的当前登录、统计缓存或系统密钥。Claude 跨电脑恢复后可能需要重新登录。", "继续", "新增 {add} · 相同数据跳过 {skip} · 同名另存 {rename}", "新增", "跳过", "同名另存", "已导出 {count} 个账户", "已导入 {count} 个账户；当前登录未改变", "备份操作失败", "清除", "没有匹配账户，试试其他关键词。", "先检查以下账户，再确认导入。同名数据另存为新别名，不覆盖现有账户，也不切换当前登录。Claude 跨电脑可能需重新登录。"],
        Language::ZhTw => ["搜尋帳戶名稱、電郵或方案", "匯入備份", "匯出全部", "全部帳戶備份", "包含 Codex 與 Claude Desktop 已儲存帳戶的登入憑證。檔案未加密，請儲存在可信的本機位置，不要分享或上傳。\n不包含未儲存的目前登入、統計快取或系統金鑰。Claude 跨電腦還原後可能需要重新登入。", "繼續", "新增 {add} · 相同資料略過 {skip} · 同名另存 {rename}", "新增", "略過", "同名另存", "已匯出 {count} 個帳戶", "已匯入 {count} 個帳戶；目前登入未改變", "備份操作失敗", "清除", "沒有符合的帳戶，請嘗試其他關鍵字。", "請先檢查以下帳戶，再確認匯入。同名資料另存為新別名，不覆蓋現有帳戶，也不切換目前登入。Claude 跨電腦可能需重新登入。"],
        Language::En => ["Search name, email or plan", "Import backup", "Export all", "All account backup", "Includes saved Codex and Claude Desktop sign-in credentials. The file is NOT encrypted. Keep it in a trusted local location; do not share or upload it.\nUnsaved live sessions, analytics caches and system keys are excluded. Claude may need sign-in again on another computer.", "Continue", "Add {add} · Skip identical {skip} · Rename conflicts {rename}", "Add", "Skip", "Rename", "Exported {count} accounts", "Imported {count} accounts; active login unchanged", "Backup failed", "Clear", "No matching accounts. Try another keyword.", "Review before importing. Conflicting names get a new alias; existing accounts and the active login stay unchanged. Claude may need sign-in again on another computer."],
        Language::Ja => ["名前・メール・プランを検索", "バックアップを読込", "すべて書出し", "全アカウントのバックアップ", "保存済み Codex と Claude Desktop の認証情報を含みます。暗号化されません。安全なローカル場所に保存し、共有やアップロードをしないでください。\n未保存のログイン、統計キャッシュ、システムキーは含みません。別のパソコンでは Claude の再ログインが必要な場合があります。", "続行", "追加 {add} · 同一データ除外 {skip} · 別名保存 {rename}", "追加", "除外", "別名保存", "{count} アカウントを書き出しました", "{count} アカウントを追加。現在のログインは変更なし", "バックアップ失敗", "クリア", "一致するアカウントがありません。", "内容を確認して読み込んでください。同名は別名で保存し、既存アカウントやログインを変更しません。別のパソコンでは Claude の再ログインが必要な場合があります。"],
        Language::Ko => ["이름, 이메일, 요금제 검색", "백업 가져오기", "모두 내보내기", "전체 계정 백업", "저장된 Codex 및 Claude Desktop 로그인 정보를 포함합니다. 파일은 암호화되지 않습니다. 안전한 로컬 위치에 보관하고 공유하거나 업로드하지 마세요.\n저장되지 않은 로그인, 통계 캐시, 시스템 키는 제외됩니다. 다른 컴퓨터에서 Claude에 다시 로그인해야 할 수 있습니다.", "계속", "추가 {add} · 동일 데이터 건너뛰기 {skip} · 이름 변경 {rename}", "추가", "건너뛰기", "이름 변경", "계정 {count}개 내보냄", "계정 {count}개 가져옴; 현재 로그인 유지", "백업 실패", "지우기", "일치하는 계정이 없습니다.", "가져올 계정을 확인하세요. 같은 이름은 새 별칭으로 저장하며 기존 계정과 현재 로그인을 변경하지 않습니다. 다른 컴퓨터에서 Claude 재로그인이 필요할 수 있습니다."],
        Language::Fr => ["Rechercher nom, e-mail ou offre", "Importer", "Tout exporter", "Sauvegarde des comptes", "Contient les identifiants Codex et Claude Desktop enregistrés. Fichier NON chiffré : conservez-le localement, sans le partager ni le téléverser.\nSessions non enregistrées, statistiques et clés système exclues. Claude peut nécessiter une reconnexion sur un autre ordinateur.", "Continuer", "Ajouter {add} · Identiques ignorés {skip} · Renommés {rename}", "Ajouter", "Ignorer", "Renommer", "{count} comptes exportés", "{count} comptes importés ; connexion active inchangée", "Échec de sauvegarde", "Effacer", "Aucun compte correspondant.", "Vérifiez avant d’importer. Les noms en conflit sont renommés. Les comptes existants et la connexion active restent inchangés. Claude peut nécessiter une reconnexion ailleurs."],
        Language::Es => ["Buscar nombre, correo o plan", "Importar copia", "Exportar todo", "Copia de todas las cuentas", "Incluye credenciales guardadas de Codex y Claude Desktop. El archivo NO está cifrado. Guárdelo en un lugar local seguro; no lo comparta ni suba.\nNo incluye sesiones sin guardar, estadísticas ni claves del sistema. Claude puede requerir iniciar sesión en otro equipo.", "Continuar", "Añadir {add} · Idénticas omitidas {skip} · Renombradas {rename}", "Añadir", "Omitir", "Renombrar", "{count} cuentas exportadas", "{count} cuentas importadas; sesión activa sin cambios", "Error de copia", "Limpiar", "No hay cuentas coincidentes.", "Revise antes de importar. Los nombres en conflicto se guardan con otro alias. Las cuentas existentes y la sesión activa no cambian. Claude puede requerir iniciar sesión en otro equipo."],
    }
}

pub fn filter(window: &MainWindow) {
    let query = window.get_account_query().to_string().trim().to_lowercase();
    let rows = window
        .get_accounts()
        .iter()
        .filter(|row| matches(row, &query))
        .collect::<Vec<_>>();
    window.set_visible_accounts(ModelRc::new(VecModel::from(rows)));
}

fn matches(row: &AccountRow, query: &str) -> bool {
    [&row.alias, &row.email, &row.plan]
        .iter()
        .any(|s| s.to_lowercase().contains(query))
}

fn panel(window: &MainWindow, heading: &str, message: &str, rows: Vec<String>, ready: bool) {
    window.set_backup_heading(heading.into());
    window.set_backup_message(message.into());
    window.set_backup_rows(ModelRc::new(VecModel::from(
        rows.into_iter().map(SharedString::from).collect::<Vec<_>>(),
    )));
    window.set_backup_ready(ready);
    window.set_backup_open(true);
}

pub fn connect(window: &MainWindow, tray: &AppTray, shared: Arc<Mutex<AppShared>>) {
    window.on_open_gcsa_link(|repositories| {
        let url = if repositories {
            "https://github.com/orgs/gcsagroup/repositories"
        } else {
            "https://gcsa.org/products"
        };
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(url).spawn();
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn();
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    });
    let pending = Arc::new(Mutex::new(None));
    let weak = window.as_weak();
    window.on_filter_accounts(move || {
        if let Some(w) = weak.upgrade() {
            filter(&w);
        }
    });
    let weak = window.as_weak();
    let action = pending.clone();
    window.on_cancel_backup(move || {
        *action.lock().unwrap() = None;
        if let Some(w) = weak.upgrade() {
            w.set_backup_open(false);
        }
    });
    let weak = window.as_weak();
    let state = shared.clone();
    let action = pending.clone();
    window.on_export_all(move || {
        let Some(w) = weak.upgrade() else {
            return;
        };
        let text = labels(state.lock().unwrap().lang);
        w.set_backup_confirm_label(confirm_label(state.lock().unwrap().lang, true).into());
        *action.lock().unwrap() = Some(Action::Export);
        panel(&w, text[3], text[4], Vec::new(), true);
    });
    let weak = window.as_weak();
    let state = shared.clone();
    let action = pending.clone();
    window.on_import_all(move || {
        let Some(w) = weak.upgrade() else {
            return;
        };
        let text = labels(state.lock().unwrap().lang);
        w.set_backup_confirm_label(confirm_label(state.lock().unwrap().lang, false).into());
        let Some(path) = rfd::FileDialog::new()
            .set_title(text[1])
            .add_filter("Account backup", &["casbackup"])
            .pick_file()
        else {
            return;
        };
        match backup::preview(&paths::app_home(), &path) {
            Ok(plan) => {
                let renamed = plan
                    .entries
                    .iter()
                    .filter(|e| e.alias != e.original_alias)
                    .count();
                let summary = text[6]
                    .replace("{add}", &plan.entries.len().to_string())
                    .replace("{skip}", &plan.skipped.to_string())
                    .replace("{rename}", &renamed.to_string());
                let rows = plan
                    .entries
                    .iter()
                    .map(|e| {
                        format!(
                            "{} · {}  {}{}",
                            e.service.label(),
                            e.original_alias,
                            if e.alias != e.original_alias {
                                "→ "
                            } else {
                                ""
                            },
                            if e.alias != e.original_alias {
                                &e.alias
                            } else {
                                text[7]
                            }
                        )
                    })
                    .collect();
                panel(&w, &summary, text[15], rows, !plan.entries.is_empty());
                *action.lock().unwrap() = Some(Action::Import(plan));
            }
            Err(error) => {
                *action.lock().unwrap() = None;
                panel(&w, text[12], &error.to_string(), Vec::new(), false);
            }
        }
    });
    let weak = window.as_weak();
    let tray_weak = tray.as_weak();
    window.on_confirm_backup(move || {
        let Some(w) = weak.upgrade() else {
            return;
        };
        let text = labels(shared.lock().unwrap().lang);
        let Some(action) = pending.lock().unwrap().take() else {
            return;
        };
        w.set_backup_ready(false);
        let result = match action {
            Action::Export => {
                let name = format!(
                    "accounts-{}.casbackup",
                    chrono::Local::now().format("%Y%m%d-%H%M%S")
                );
                let Some(path) = rfd::FileDialog::new()
                    .set_title(text[2])
                    .set_file_name(name)
                    .add_filter("Account backup", &["casbackup"])
                    .save_file()
                else {
                    w.set_backup_open(false);
                    return;
                };
                backup::export(&paths::app_home(), &path).map(|count| {
                    (
                        text[10].replace("{count}", &count.to_string()),
                        path.display().to_string(),
                    )
                })
            }
            Action::Import(plan) => backup::apply(&paths::app_home(), &plan).map(|count| {
                w.set_account_query(SharedString::new());
                (
                    text[11].replace("{count}", &count.to_string()),
                    String::new(),
                )
            }),
        };
        match result {
            Ok((message, path)) => {
                if let Some(t) = tray_weak.upgrade() {
                    refresh_ui(&w, &t, &shared);
                }
                w.set_status_text(message.clone().into());
                panel(&w, &message, &path, Vec::new(), false);
            }
            Err(error) => panel(&w, text[12], &error.to_string(), Vec::new(), false),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_matches_name_email_and_plan_without_case() {
        let row = AccountRow {
            alias: "Work".into(),
            email: "USER@example.com".into(),
            plan: "Pro".into(),
            ..Default::default()
        };
        for query in ["", "work", "user@", "pro"] {
            assert!(matches(&row, query));
        }
        assert!(!matches(&row, "missing"));
    }
}
