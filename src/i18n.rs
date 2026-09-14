use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    En,
    ZhCn,
    ZhTw,
    Ja,
    Ko,
    Fr,
    Es,
}

impl Language {
    pub fn system_default() -> Self {
        #[cfg(target_os = "macos")]
        let platform_locale = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleLocale"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
            .unwrap_or_default();
        #[cfg(not(target_os = "macos"))]
        let platform_locale = String::new();

        let lang = if platform_locale.trim().is_empty() {
            std::env::var("LANG")
                .or_else(|_| std::env::var("LC_ALL"))
                .unwrap_or_default()
        } else {
            platform_locale
        }
        .to_lowercase();
        if lang.contains("zh_cn")
            || lang.contains("zh_sg")
            || lang.contains("zh-hans")
            || lang.contains("zh_hans")
        {
            Self::ZhCn
        } else if lang.contains("zh_tw") || lang.contains("zh-hant") || lang.contains("zh_hk") {
            Self::ZhTw
        } else if lang.starts_with("ja") {
            Self::Ja
        } else if lang.starts_with("ko") {
            Self::Ko
        } else if lang.starts_with("fr") {
            Self::Fr
        } else if lang.starts_with("es") {
            Self::Es
        } else {
            Self::En
        }
    }

    pub fn preferred_or_system() -> Self {
        let path = crate::paths::app_home().join("language");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|value| Self::from_code(value.trim()))
            .unwrap_or_else(Self::system_default)
    }

    pub fn save_preferred(self) -> crate::error::AppResult<()> {
        let path = crate::paths::app_home().join("language");
        crate::profile::atomic_write(&path, self.code().as_bytes())
    }

    fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhCn => "zh-cn",
            Self::ZhTw => "zh-tw",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Fr => "fr",
            Self::Es => "es",
        }
    }

    fn from_code(value: &str) -> Option<Self> {
        match value {
            "en" => Some(Self::En),
            "zh-cn" => Some(Self::ZhCn),
            "zh-tw" => Some(Self::ZhTw),
            "ja" => Some(Self::Ja),
            "ko" => Some(Self::Ko),
            "fr" => Some(Self::Fr),
            "es" => Some(Self::Es),
            _ => None,
        }
    }
}

static STRINGS: OnceLock<HashMap<(Language, &'static str), &'static str>> = OnceLock::new();

fn strings() -> &'static HashMap<(Language, &'static str), &'static str> {
    STRINGS.get_or_init(|| {
        let mut m = HashMap::new();

        // English (fallback)
        m.insert((Language::En, "app.title"), "Codex Account Switch");
        m.insert((Language::En, "tray.refresh"), "Refresh Usage");
        m.insert((Language::En, "tray.switch"), "Switch Account");
        m.insert((Language::En, "tray.quit"), "Quit");
        m.insert((Language::En, "btn.refresh_usage"), "Refresh Usage");
        m.insert((Language::En, "btn.refresh_list"), "Refresh List");
        m.insert((Language::En, "btn.quit"), "Quit");
        m.insert((Language::En, "active.title"), "ACTIVE");
        m.insert((Language::En, "active.no_account"), "No account selected");
        m.insert((Language::En, "active.hint"), "Save or switch an account to see identity");
        m.insert((Language::En, "active.restart_hint"), "Restart Codex after switching to take effect");
        m.insert((Language::En, "token.title"), "TOKEN ACTIVITY");
        m.insert((Language::En, "token.lifetime"), "Lifetime");
        m.insert((Language::En, "token.peak"), "Peak day");
        m.insert((Language::En, "token.streak"), "Streak");
        m.insert((Language::En, "token.longest"), "Longest");
        m.insert((Language::En, "token.last_seven"), "LAST 7 DAYS");
        m.insert((Language::En, "import.tab.login"), "Browser Login");
        m.insert((Language::En, "import.tab.save"), "Save Current");
        m.insert((Language::En, "import.tab.path"), "From Path");
        m.insert((Language::En, "import.tab.paste"), "Paste JSON");
        m.insert((Language::En, "login.hint"), "Sign in with ChatGPT OAuth; the profile is saved automatically after completion");
        m.insert((Language::En, "login.alias_placeholder"), "Optional alias, defaults to email prefix");
        m.insert((Language::En, "login.open"), "Open Browser to Login");
        m.insert((Language::En, "login.running"), "Logging in…");
        m.insert((Language::En, "login.cancel"), "Cancel");
        m.insert((Language::En, "save.alias_placeholder"), "Alias, e.g. work");
        m.insert((Language::En, "save.button"), "Save Current");
        m.insert((Language::En, "save.import"), "Import Current");
        m.insert((Language::En, "path.placeholder"), "Paste full directory path to open…");
        m.insert((Language::En, "path.open"), "Open Directory…");
        m.insert((Language::En, "paste.placeholder"), "Paste full auth.json content here (supports ```json blocks)…");
        m.insert((Language::En, "paste.alias_placeholder"), "Optional alias, defaults to email prefix");
        m.insert((Language::En, "paste.import"), "Import from Paste");
        m.insert((Language::En, "paste.clipboard"), "From Clipboard");
        m.insert((Language::En, "rename.title"), "Rename");
        m.insert((Language::En, "rename.new_placeholder"), "New alias");
        m.insert((Language::En, "rename.confirm"), "Confirm");
        m.insert((Language::En, "rename.cancel"), "Cancel");
        m.insert((Language::En, "profiles.title"), "PROFILES");
        m.insert((Language::En, "profiles.empty"), "No saved accounts yet");
        m.insert((Language::En, "profiles.empty_hint"), "Use \"Save Current\" or import auth.json from path");
        m.insert((Language::En, "profiles.count"), "saved");
        m.insert((Language::En, "btn.switch"), "Switch");
        m.insert((Language::En, "btn.restart"), "Restart");
        m.insert((Language::En, "btn.rename"), "Rename");
        m.insert((Language::En, "btn.delete"), "Delete");
        m.insert((Language::En, "label.in_use"), "IN USE");
        m.insert((Language::En, "label.no_email"), "No email");
        m.insert((Language::En, "gauge.5h"), "5H WINDOW");
        m.insert((Language::En, "gauge.weekly"), "WEEKLY");
        m.insert(
            (Language::En, "claude.session_hint"),
            "Switching restarts Claude Desktop automatically",
        );
        m.insert(
            (Language::En, "claude.login_hint"),
            "Open Claude Desktop, sign in, then use Save Current",
        );
        m.insert(
            (Language::En, "claude.paste_placeholder"),
            "Paste an exported Claude credentials snapshot here…",
        );

        // 简体中文
        m.insert((Language::ZhCn, "app.title"), "Codex Account Switch");
        m.insert((Language::ZhCn, "tray.refresh"), "刷新额度");
        m.insert((Language::ZhCn, "tray.switch"), "切换账号");
        m.insert((Language::ZhCn, "tray.quit"), "退出");
        m.insert((Language::ZhCn, "btn.refresh_usage"), "刷新额度");
        m.insert((Language::ZhCn, "btn.refresh_list"), "刷新列表");
        m.insert((Language::ZhCn, "btn.quit"), "退出");
        m.insert((Language::ZhCn, "active.title"), "当前账号");
        m.insert((Language::ZhCn, "active.no_account"), "未选择账号");
        m.insert((Language::ZhCn, "active.hint"), "保存或切换一个账号后显示标识");
        m.insert((Language::ZhCn, "active.restart_hint"), "切换后需重启 Codex 才生效");
        m.insert((Language::ZhCn, "token.title"), "TOKEN 活动");
        m.insert((Language::ZhCn, "token.lifetime"), "累计用量");
        m.insert((Language::ZhCn, "token.peak"), "峰值日");
        m.insert((Language::ZhCn, "token.streak"), "连续使用");
        m.insert((Language::ZhCn, "token.longest"), "最长会话");
        m.insert((Language::ZhCn, "token.last_seven"), "最近 7 天");
        m.insert((Language::ZhCn, "import.tab.login"), "浏览器登录");
        m.insert((Language::ZhCn, "import.tab.save"), "保存当前");
        m.insert((Language::ZhCn, "import.tab.path"), "从路径");
        m.insert((Language::ZhCn, "import.tab.paste"), "粘贴文本");
        m.insert((Language::ZhCn, "login.hint"), "通过 ChatGPT 官方授权登录，完成后自动保存到 Profiles");
        m.insert((Language::ZhCn, "login.alias_placeholder"), "可选别名，留空则用邮箱自动生成");
        m.insert((Language::ZhCn, "login.open"), "打开浏览器登录");
        m.insert((Language::ZhCn, "login.running"), "登录中…");
        m.insert((Language::ZhCn, "login.cancel"), "取消");
        m.insert((Language::ZhCn, "save.alias_placeholder"), "别名，如 work");
        m.insert((Language::ZhCn, "save.button"), "保存当前");
        m.insert((Language::ZhCn, "save.import"), "导入当前");
        m.insert((Language::ZhCn, "path.placeholder"), "粘贴完整目录路径后打开…");
        m.insert((Language::ZhCn, "path.open"), "打开目录…");
        m.insert((Language::ZhCn, "paste.placeholder"), "在此粘贴 auth.json 全文（支持 ```json 代码块）…");
        m.insert((Language::ZhCn, "paste.alias_placeholder"), "可选别名，留空则用邮箱自动生成");
        m.insert((Language::ZhCn, "paste.import"), "粘贴导入");
        m.insert((Language::ZhCn, "paste.clipboard"), "从剪贴板");
        m.insert((Language::ZhCn, "rename.title"), "重命名");
        m.insert((Language::ZhCn, "rename.new_placeholder"), "新别名");
        m.insert((Language::ZhCn, "rename.confirm"), "确认");
        m.insert((Language::ZhCn, "rename.cancel"), "取消");
        m.insert((Language::ZhCn, "profiles.title"), "账号列表");
        m.insert((Language::ZhCn, "profiles.empty"), "还没有保存的账号");
        m.insert((Language::ZhCn, "profiles.empty_hint"), "先「保存当前」或从路径导入 auth.json");
        m.insert((Language::ZhCn, "profiles.count"), "个已保存");
        m.insert((Language::ZhCn, "btn.switch"), "切换");
        m.insert((Language::ZhCn, "btn.restart"), "重启");
        m.insert((Language::ZhCn, "btn.rename"), "改名");
        m.insert((Language::ZhCn, "btn.delete"), "删除");
        m.insert((Language::ZhCn, "label.in_use"), "使用中");
        m.insert((Language::ZhCn, "label.no_email"), "无邮箱");
        m.insert((Language::ZhCn, "gauge.5h"), "5 小时窗口");
        m.insert((Language::ZhCn, "gauge.weekly"), "每周窗口");
        m.insert(
            (Language::ZhCn, "claude.session_hint"),
            "切换账号时会自动重启 Claude Desktop",
        );
        m.insert(
            (Language::ZhCn, "claude.login_hint"),
            "打开 Claude Desktop 登录账号，然后回到这里「保存当前」",
        );
        m.insert(
            (Language::ZhCn, "claude.paste_placeholder"),
            "在此粘贴导出的 Claude 凭据快照…",
        );

        // 繁體中文
        m.insert((Language::ZhTw, "app.title"), "Codex Account Switch");
        m.insert((Language::ZhTw, "tray.refresh"), "刷新額度");
        m.insert((Language::ZhTw, "tray.switch"), "切換帳號");
        m.insert((Language::ZhTw, "tray.quit"), "退出");
        m.insert((Language::ZhTw, "btn.refresh_usage"), "刷新額度");
        m.insert((Language::ZhTw, "btn.refresh_list"), "刷新列表");
        m.insert((Language::ZhTw, "btn.quit"), "退出");
        m.insert((Language::ZhTw, "active.title"), "當前帳號");
        m.insert((Language::ZhTw, "active.no_account"), "未選擇帳號");
        m.insert((Language::ZhTw, "active.hint"), "保存或切換一個帳號後顯示標識");
        m.insert((Language::ZhTw, "active.restart_hint"), "切換後需重啟 Codex 才生效");
        m.insert((Language::ZhTw, "token.title"), "TOKEN 活動");
        m.insert((Language::ZhTw, "token.lifetime"), "累計用量");
        m.insert((Language::ZhTw, "token.peak"), "峰值日");
        m.insert((Language::ZhTw, "token.streak"), "連續使用");
        m.insert((Language::ZhTw, "token.longest"), "最長會話");
        m.insert((Language::ZhTw, "token.last_seven"), "最近 7 天");
        m.insert((Language::ZhTw, "import.tab.login"), "瀏覽器登錄");
        m.insert((Language::ZhTw, "import.tab.save"), "保存當前");
        m.insert((Language::ZhTw, "import.tab.path"), "從路徑");
        m.insert((Language::ZhTw, "import.tab.paste"), "粘貼文本");
        m.insert((Language::ZhTw, "login.hint"), "通過 ChatGPT 官方授權登錄，完成後自動保存到 Profiles");
        m.insert((Language::ZhTw, "login.alias_placeholder"), "可選別名，留空則用郵箱自動生成");
        m.insert((Language::ZhTw, "login.open"), "打開瀏覽器登錄");
        m.insert((Language::ZhTw, "login.running"), "登錄中…");
        m.insert((Language::ZhTw, "login.cancel"), "取消");
        m.insert((Language::ZhTw, "save.alias_placeholder"), "別名，如 work");
        m.insert((Language::ZhTw, "save.button"), "保存當前");
        m.insert((Language::ZhTw, "save.import"), "導入當前");
        m.insert((Language::ZhTw, "path.placeholder"), "粘貼完整目錄路徑後打開…");
        m.insert((Language::ZhTw, "path.open"), "打開目錄…");
        m.insert((Language::ZhTw, "paste.placeholder"), "在此粘貼 auth.json 全文（支持 ```json 代碼塊）…");
        m.insert((Language::ZhTw, "paste.alias_placeholder"), "可選別名，留空則用郵箱自動生成");
        m.insert((Language::ZhTw, "paste.import"), "粘貼導入");
        m.insert((Language::ZhTw, "paste.clipboard"), "從剪貼板");
        m.insert((Language::ZhTw, "rename.title"), "重命名");
        m.insert((Language::ZhTw, "rename.new_placeholder"), "新別名");
        m.insert((Language::ZhTw, "rename.confirm"), "確認");
        m.insert((Language::ZhTw, "rename.cancel"), "取消");
        m.insert((Language::ZhTw, "profiles.title"), "帳號列表");
        m.insert((Language::ZhTw, "profiles.empty"), "還沒有保存的帳號");
        m.insert((Language::ZhTw, "profiles.empty_hint"), "先「保存當前」或從路徑導入 auth.json");
        m.insert((Language::ZhTw, "profiles.count"), "個已保存");
        m.insert((Language::ZhTw, "btn.switch"), "切換");
        m.insert((Language::ZhTw, "btn.restart"), "重啟");
        m.insert((Language::ZhTw, "btn.rename"), "改名");
        m.insert((Language::ZhTw, "btn.delete"), "刪除");
        m.insert((Language::ZhTw, "label.in_use"), "使用中");
        m.insert((Language::ZhTw, "label.no_email"), "無郵箱");
        m.insert((Language::ZhTw, "gauge.5h"), "5 小時窗口");
        m.insert((Language::ZhTw, "gauge.weekly"), "每週窗口");
        m.insert(
            (Language::ZhTw, "claude.session_hint"),
            "切換帳號時會自動重新啟動 Claude Desktop",
        );
        m.insert(
            (Language::ZhTw, "claude.login_hint"),
            "開啟 Claude Desktop 登入帳號，然後回到這裡「保存當前」",
        );
        m.insert(
            (Language::ZhTw, "claude.paste_placeholder"),
            "在此貼上匯出的 Claude 憑證快照…",
        );

        // 日本語
        m.insert((Language::Ja, "app.title"), "Codex Account Switch");
        m.insert((Language::Ja, "tray.refresh"), "使用量を更新");
        m.insert((Language::Ja, "tray.switch"), "アカウント切替");
        m.insert((Language::Ja, "tray.quit"), "終了");
        m.insert((Language::Ja, "btn.refresh_usage"), "使用量を更新");
        m.insert((Language::Ja, "btn.refresh_list"), "リストを更新");
        m.insert((Language::Ja, "btn.quit"), "終了");
        m.insert((Language::Ja, "active.title"), "アクティブ");
        m.insert((Language::Ja, "active.no_account"), "アカウント未選択");
        m.insert((Language::Ja, "active.hint"), "アカウントを保存または切り替えると識別情報が表示されます");
        m.insert((Language::Ja, "active.restart_hint"), "切り替え後は Codex の再起動が必要です");
        m.insert((Language::Ja, "token.title"), "トークン使用量");
        m.insert((Language::Ja, "token.lifetime"), "累計使用量");
        m.insert((Language::Ja, "token.peak"), "ピーク日");
        m.insert((Language::Ja, "token.streak"), "連続使用");
        m.insert((Language::Ja, "token.longest"), "最長セッション");
        m.insert((Language::Ja, "token.last_seven"), "直近 7 日間");
        m.insert((Language::Ja, "import.tab.login"), "ブラウザでログイン");
        m.insert((Language::Ja, "import.tab.save"), "現在を保存");
        m.insert((Language::Ja, "import.tab.path"), "パスから");
        m.insert((Language::Ja, "import.tab.paste"), "テキスト貼付");
        m.insert((Language::Ja, "login.hint"), "ChatGPT 公式認証でログインし、完了後に自動で Profiles に保存されます");
        m.insert((Language::Ja, "login.alias_placeholder"), "オプションのエイリアス（空欄はメールから自動生成）");
        m.insert((Language::Ja, "login.open"), "ブラウザでログイン");
        m.insert((Language::Ja, "login.running"), "ログイン中…");
        m.insert((Language::Ja, "login.cancel"), "キャンセル");
        m.insert((Language::Ja, "save.alias_placeholder"), "エイリアス（例: work）");
        m.insert((Language::Ja, "save.button"), "現在を保存");
        m.insert((Language::Ja, "save.import"), "現在をインポート");
        m.insert((Language::Ja, "path.placeholder"), "完全なディレクトリパスを貼って開く…");
        m.insert((Language::Ja, "path.open"), "ディレクトリを開く…");
        m.insert((Language::Ja, "paste.placeholder"), "auth.json の全文をここに貼り付け（```json ブロック可）…");
        m.insert((Language::Ja, "paste.alias_placeholder"), "オプションのエイリアス（空欄はメールから自動生成）");
        m.insert((Language::Ja, "paste.import"), "貼り付けでインポート");
        m.insert((Language::Ja, "paste.clipboard"), "クリップボードから");
        m.insert((Language::Ja, "rename.title"), "名前変更");
        m.insert((Language::Ja, "rename.new_placeholder"), "新しいエイリアス");
        m.insert((Language::Ja, "rename.confirm"), "確認");
        m.insert((Language::Ja, "rename.cancel"), "キャンセル");
        m.insert((Language::Ja, "profiles.title"), "プロファイル");
        m.insert((Language::Ja, "profiles.empty"), "保存済みアカウントがありません");
        m.insert((Language::Ja, "profiles.empty_hint"), "「現在を保存」またはパスから auth.json をインポートしてください");
        m.insert((Language::Ja, "profiles.count"), "件保存済み");
        m.insert((Language::Ja, "btn.switch"), "切替");
        m.insert((Language::Ja, "btn.restart"), "再起動");
        m.insert((Language::Ja, "btn.rename"), "改名");
        m.insert((Language::Ja, "btn.delete"), "削除");
        m.insert((Language::Ja, "label.in_use"), "使用中");
        m.insert((Language::Ja, "label.no_email"), "メールなし");
        m.insert((Language::Ja, "gauge.5h"), "5時間枠");
        m.insert((Language::Ja, "gauge.weekly"), "週間枠");
        m.insert(
            (Language::Ja, "claude.session_hint"),
            "アカウントを切り替えると Claude Desktop が自動的に再起動します",
        );
        m.insert(
            (Language::Ja, "claude.login_hint"),
            "Claude Desktop を開いてログインし、「現在を保存」を選択してください",
        );
        m.insert(
            (Language::Ja, "claude.paste_placeholder"),
            "エクスポートした Claude 認証情報のスナップショットをここに貼り付け…",
        );

        // 한국어
        m.insert((Language::Ko, "app.title"), "Codex Account Switch");
        m.insert((Language::Ko, "tray.refresh"), "사용량 새로고침");
        m.insert((Language::Ko, "tray.switch"), "계정 전환");
        m.insert((Language::Ko, "tray.quit"), "종료");
        m.insert((Language::Ko, "btn.refresh_usage"), "사용량 새로고침");
        m.insert((Language::Ko, "btn.refresh_list"), "목록 새로고침");
        m.insert((Language::Ko, "btn.quit"), "종료");
        m.insert((Language::Ko, "active.title"), "활성");
        m.insert((Language::Ko, "active.no_account"), "계정 미선택");
        m.insert((Language::Ko, "active.hint"), "계정을 저장하거나 전환하면 식별 정보가 표시됩니다");
        m.insert((Language::Ko, "active.restart_hint"), "전환 후 Codex를 재시작해야 적용됩니다");
        m.insert((Language::Ko, "token.title"), "토큰 사용량");
        m.insert((Language::Ko, "token.lifetime"), "누적 사용량");
        m.insert((Language::Ko, "token.peak"), "최고 일일");
        m.insert((Language::Ko, "token.streak"), "연속 사용");
        m.insert((Language::Ko, "token.longest"), "최장 세션");
        m.insert((Language::Ko, "token.last_seven"), "최근 7일");
        m.insert((Language::Ko, "import.tab.login"), "브라우저 로그인");
        m.insert((Language::Ko, "import.tab.save"), "현재 저장");
        m.insert((Language::Ko, "import.tab.path"), "경로에서");
        m.insert((Language::Ko, "import.tab.paste"), "텍스트 붙여넣기");
        m.insert((Language::Ko, "login.hint"), "ChatGPT 공식 인증으로 로그인하면 완료 후 자동으로 Profiles에 저장됩니다");
        m.insert((Language::Ko, "login.alias_placeholder"), "선택적 별칭(비우면 이메일에서 자동 생성)");
        m.insert((Language::Ko, "login.open"), "브라우저로 로그인");
        m.insert((Language::Ko, "login.running"), "로그인 중…");
        m.insert((Language::Ko, "login.cancel"), "취소");
        m.insert((Language::Ko, "save.alias_placeholder"), "별칭(예: work)");
        m.insert((Language::Ko, "save.button"), "현재 저장");
        m.insert((Language::Ko, "save.import"), "현재 가져오기");
        m.insert((Language::Ko, "path.placeholder"), "전체 디렉터리 경로를 붙여넣고 열기…");
        m.insert((Language::Ko, "path.open"), "디렉터리 열기…");
        m.insert((Language::Ko, "paste.placeholder"), "여기에 auth.json 전체를 붙여넣으세요(```json 블록 지원)…");
        m.insert((Language::Ko, "paste.alias_placeholder"), "선택적 별칭(비우면 이메일에서 자동 생성)");
        m.insert((Language::Ko, "paste.import"), "붙여넣기 가져오기");
        m.insert((Language::Ko, "paste.clipboard"), "클립보드에서");
        m.insert((Language::Ko, "rename.title"), "이름 변경");
        m.insert((Language::Ko, "rename.new_placeholder"), "새 별칭");
        m.insert((Language::Ko, "rename.confirm"), "확인");
        m.insert((Language::Ko, "rename.cancel"), "취소");
        m.insert((Language::Ko, "profiles.title"), "프로필");
        m.insert((Language::Ko, "profiles.empty"), "저장된 계정이 없습니다");
        m.insert((Language::Ko, "profiles.empty_hint"), "「현재 저장」 또는 경로에서 auth.json 가져오기");
        m.insert((Language::Ko, "profiles.count"), "개 저장됨");
        m.insert((Language::Ko, "btn.switch"), "전환");
        m.insert((Language::Ko, "btn.restart"), "재시작");
        m.insert((Language::Ko, "btn.rename"), "이름 변경");
        m.insert((Language::Ko, "btn.delete"), "삭제");
        m.insert((Language::Ko, "label.in_use"), "사용 중");
        m.insert((Language::Ko, "label.no_email"), "이메일 없음");
        m.insert((Language::Ko, "gauge.5h"), "5시간 창");
        m.insert((Language::Ko, "gauge.weekly"), "주간 창");
        m.insert(
            (Language::Ko, "claude.session_hint"),
            "계정을 전환하면 Claude Desktop이 자동으로 다시 시작됩니다",
        );
        m.insert(
            (Language::Ko, "claude.login_hint"),
            "Claude Desktop을 열어 로그인한 다음 현재 저장을 누르세요",
        );
        m.insert(
            (Language::Ko, "claude.paste_placeholder"),
            "내보낸 Claude 자격 증명 스냅샷을 여기에 붙여넣으세요…",
        );

        // Français
        m.insert((Language::Fr, "app.title"), "Codex Account Switch");
        m.insert((Language::Fr, "tray.refresh"), "Actualiser l'utilisation");
        m.insert((Language::Fr, "tray.switch"), "Changer de compte");
        m.insert((Language::Fr, "tray.quit"), "Quitter");
        m.insert((Language::Fr, "btn.refresh_usage"), "Actualiser l'utilisation");
        m.insert((Language::Fr, "btn.refresh_list"), "Actualiser la liste");
        m.insert((Language::Fr, "btn.quit"), "Quitter");
        m.insert((Language::Fr, "active.title"), "ACTIF");
        m.insert((Language::Fr, "active.no_account"), "Aucun compte sélectionné");
        m.insert((Language::Fr, "active.hint"), "Enregistrez ou changez de compte pour afficher l'identité");
        m.insert((Language::Fr, "active.restart_hint"), "Redémarrez Codex après le changement pour appliquer");
        m.insert((Language::Fr, "token.title"), "ACTIVITÉ TOKEN");
        m.insert((Language::Fr, "token.lifetime"), "Cumul");
        m.insert((Language::Fr, "token.peak"), "Pic journalier");
        m.insert((Language::Fr, "token.streak"), "Série");
        m.insert((Language::Fr, "token.longest"), "Plus longue session");
        m.insert((Language::Fr, "token.last_seven"), "7 DERNIERS JOURS");
        m.insert((Language::Fr, "import.tab.login"), "Connexion navigateur");
        m.insert((Language::Fr, "import.tab.save"), "Enregistrer actuel");
        m.insert((Language::Fr, "import.tab.path"), "Depuis chemin");
        m.insert((Language::Fr, "import.tab.paste"), "Coller texte");
        m.insert((Language::Fr, "login.hint"), "Connectez-vous via l'authentification ChatGPT officielle ; le profil est enregistré automatiquement");
        m.insert((Language::Fr, "login.alias_placeholder"), "Alias optionnel, sinon généré depuis l'email");
        m.insert((Language::Fr, "login.open"), "Ouvrir le navigateur pour se connecter");
        m.insert((Language::Fr, "login.running"), "Connexion…");
        m.insert((Language::Fr, "login.cancel"), "Annuler");
        m.insert((Language::Fr, "save.alias_placeholder"), "Alias, ex. work");
        m.insert((Language::Fr, "save.button"), "Enregistrer actuel");
        m.insert((Language::Fr, "save.import"), "Importer actuel");
        m.insert((Language::Fr, "path.placeholder"), "Collez le chemin complet du dossier puis ouvrez…");
        m.insert((Language::Fr, "path.open"), "Ouvrir le dossier…");
        m.insert((Language::Fr, "paste.placeholder"), "Collez le contenu complet de auth.json ici (blocs ```json acceptés)…");
        m.insert((Language::Fr, "paste.alias_placeholder"), "Alias optionnel, sinon généré depuis l'email");
        m.insert((Language::Fr, "paste.import"), "Importer depuis le collage");
        m.insert((Language::Fr, "paste.clipboard"), "Depuis le presse-papiers");
        m.insert((Language::Fr, "rename.title"), "Renommer");
        m.insert((Language::Fr, "rename.new_placeholder"), "Nouvel alias");
        m.insert((Language::Fr, "rename.confirm"), "Confirmer");
        m.insert((Language::Fr, "rename.cancel"), "Annuler");
        m.insert((Language::Fr, "profiles.title"), "PROFILS");
        m.insert((Language::Fr, "profiles.empty"), "Aucun compte enregistré");
        m.insert((Language::Fr, "profiles.empty_hint"), "Utilisez « Enregistrer actuel » ou importez auth.json depuis un chemin");
        m.insert((Language::Fr, "profiles.count"), "enregistrés");
        m.insert((Language::Fr, "btn.switch"), "Changer");
        m.insert((Language::Fr, "btn.restart"), "Redémarrer");
        m.insert((Language::Fr, "btn.rename"), "Renommer");
        m.insert((Language::Fr, "btn.delete"), "Supprimer");
        m.insert((Language::Fr, "label.in_use"), "EN COURS");
        m.insert((Language::Fr, "label.no_email"), "Pas d'email");
        m.insert((Language::Fr, "gauge.5h"), "FENÊTRE 5H");
        m.insert((Language::Fr, "gauge.weekly"), "HEBDOMADAIRE");
        m.insert(
            (Language::Fr, "claude.session_hint"),
            "Le changement de compte redémarre automatiquement Claude Desktop",
        );
        m.insert(
            (Language::Fr, "claude.login_hint"),
            "Ouvrez Claude Desktop, connectez-vous, puis enregistrez le compte actuel",
        );
        m.insert(
            (Language::Fr, "claude.paste_placeholder"),
            "Collez ici l'instantané exporté des identifiants Claude…",
        );

        // Español
        m.insert((Language::Es, "app.title"), "Codex Account Switch");
        m.insert((Language::Es, "tray.refresh"), "Actualizar uso");
        m.insert((Language::Es, "tray.switch"), "Cambiar cuenta");
        m.insert((Language::Es, "tray.quit"), "Salir");
        m.insert((Language::Es, "btn.refresh_usage"), "Actualizar uso");
        m.insert((Language::Es, "btn.refresh_list"), "Actualizar lista");
        m.insert((Language::Es, "btn.quit"), "Salir");
        m.insert((Language::Es, "active.title"), "ACTIVO");
        m.insert((Language::Es, "active.no_account"), "No hay cuenta seleccionada");
        m.insert((Language::Es, "active.hint"), "Guarda o cambia una cuenta para ver la identidad");
        m.insert((Language::Es, "active.restart_hint"), "Reinicia Codex después de cambiar para aplicar");
        m.insert((Language::Es, "token.title"), "ACTIVIDAD DE TOKEN");
        m.insert((Language::Es, "token.lifetime"), "Acumulado");
        m.insert((Language::Es, "token.peak"), "Día pico");
        m.insert((Language::Es, "token.streak"), "Racha");
        m.insert((Language::Es, "token.longest"), "Sesión más larga");
        m.insert((Language::Es, "token.last_seven"), "ÚLTIMOS 7 DÍAS");
        m.insert((Language::Es, "import.tab.login"), "Iniciar sesión en navegador");
        m.insert((Language::Es, "import.tab.save"), "Guardar actual");
        m.insert((Language::Es, "import.tab.path"), "Desde ruta");
        m.insert((Language::Es, "import.tab.paste"), "Pegar texto");
        m.insert((Language::Es, "login.hint"), "Inicia sesión con la autorización oficial de ChatGPT; el perfil se guarda automáticamente al terminar");
        m.insert((Language::Es, "login.alias_placeholder"), "Alias opcional, si no se genera desde el email");
        m.insert((Language::Es, "login.open"), "Abrir navegador para iniciar sesión");
        m.insert((Language::Es, "login.running"), "Iniciando sesión…");
        m.insert((Language::Es, "login.cancel"), "Cancelar");
        m.insert((Language::Es, "save.alias_placeholder"), "Alias, ej. work");
        m.insert((Language::Es, "save.button"), "Guardar actual");
        m.insert((Language::Es, "save.import"), "Importar actual");
        m.insert((Language::Es, "path.placeholder"), "Pega la ruta completa del directorio y abre…");
        m.insert((Language::Es, "path.open"), "Abrir directorio…");
        m.insert((Language::Es, "paste.placeholder"), "Pega aquí el contenido completo de auth.json (admite bloques ```json)…");
        m.insert((Language::Es, "paste.alias_placeholder"), "Alias opcional, si no se genera desde el email");
        m.insert((Language::Es, "paste.import"), "Importar desde pegado");
        m.insert((Language::Es, "paste.clipboard"), "Desde portapapeles");
        m.insert((Language::Es, "rename.title"), "Renombrar");
        m.insert((Language::Es, "rename.new_placeholder"), "Nuevo alias");
        m.insert((Language::Es, "rename.confirm"), "Confirmar");
        m.insert((Language::Es, "rename.cancel"), "Cancelar");
        m.insert((Language::Es, "profiles.title"), "PERFILES");
        m.insert((Language::Es, "profiles.empty"), "Aún no hay cuentas guardadas");
        m.insert((Language::Es, "profiles.empty_hint"), "Usa «Guardar actual» o importa auth.json desde una ruta");
        m.insert((Language::Es, "profiles.count"), "guardadas");
        m.insert((Language::Es, "btn.switch"), "Cambiar");
        m.insert((Language::Es, "btn.restart"), "Reiniciar");
        m.insert((Language::Es, "btn.rename"), "Renombrar");
        m.insert((Language::Es, "btn.delete"), "Eliminar");
        m.insert((Language::Es, "label.in_use"), "EN USO");
        m.insert((Language::Es, "label.no_email"), "Sin email");
        m.insert((Language::Es, "gauge.5h"), "VENTANA 5H");
        m.insert((Language::Es, "gauge.weekly"), "SEMANAL");
        m.insert(
            (Language::Es, "claude.session_hint"),
            "Cambiar de cuenta reinicia Claude Desktop automáticamente",
        );
        m.insert(
            (Language::Es, "claude.login_hint"),
            "Abre Claude Desktop, inicia sesión y guarda la cuenta actual",
        );
        m.insert(
            (Language::Es, "claude.paste_placeholder"),
            "Pega aquí la instantánea exportada de credenciales de Claude…",
        );

        // Status messages
        m.insert((Language::En, "status.list_refreshed"), "List refreshed");
        m.insert((Language::ZhCn, "status.list_refreshed"), "列表已刷新");
        m.insert((Language::ZhTw, "status.list_refreshed"), "列表已刷新");
        m.insert((Language::Ja, "status.list_refreshed"), "リストを更新しました");
        m.insert((Language::Ko, "status.list_refreshed"), "목록 새로고침됨");
        m.insert((Language::Fr, "status.list_refreshed"), "Liste actualisée");
        m.insert((Language::Es, "status.list_refreshed"), "Lista actualizada");

        m.insert((Language::En, "status.usage_updated"), "Usage updated");
        m.insert((Language::ZhCn, "status.usage_updated"), "额度已更新");
        m.insert((Language::ZhTw, "status.usage_updated"), "額度已更新");
        m.insert((Language::Ja, "status.usage_updated"), "使用量を更新しました");
        m.insert((Language::Ko, "status.usage_updated"), "사용량 업데이트됨");
        m.insert((Language::Fr, "status.usage_updated"), "Consommation actualisée");
        m.insert((Language::Es, "status.usage_updated"), "Uso actualizado");

        m.insert((Language::En, "status.already_current"), "Already current account");
        m.insert((Language::ZhCn, "status.already_current"), "已是当前账号");
        m.insert((Language::ZhTw, "status.already_current"), "已是當前賬號");
        m.insert((Language::Ja, "status.already_current"), "すでに現在のアカウントです");
        m.insert((Language::Ko, "status.already_current"), "이미 현재 계정입니다");
        m.insert((Language::Fr, "status.already_current"), "Compte déjà actif");
        m.insert((Language::Es, "status.already_current"), "Ya es la cuenta actual");

        m.insert((Language::En, "status.switched"), "Switched →");
        m.insert((Language::ZhCn, "status.switched"), "已切换 →");
        m.insert((Language::ZhTw, "status.switched"), "已切換 →");
        m.insert((Language::Ja, "status.switched"), "切り替え →");
        m.insert((Language::Ko, "status.switched"), "전환됨 →");
        m.insert((Language::Fr, "status.switched"), "Basculé →");
        m.insert((Language::Es, "status.switched"), "Cambiado →");

        m.insert((Language::En, "status.switch_failed"), "Switch failed");
        m.insert((Language::ZhCn, "status.switch_failed"), "切换失败");
        m.insert((Language::ZhTw, "status.switch_failed"), "切換失敗");
        m.insert((Language::Ja, "status.switch_failed"), "切り替え失敗");
        m.insert((Language::Ko, "status.switch_failed"), "전환 실패");
        m.insert((Language::Fr, "status.switch_failed"), "Échec du changement");
        m.insert((Language::Es, "status.switch_failed"), "Error al cambiar");

        m.insert((Language::En, "status.restart_codex"), "Please restart Codex to take effect");
        m.insert((Language::ZhCn, "status.restart_codex"), "请重启 Codex 以生效");
        m.insert((Language::ZhTw, "status.restart_codex"), "請重啟 Codex 以生效");
        m.insert((Language::Ja, "status.restart_codex"), "Codexを再起動してください");
        m.insert((Language::Ko, "status.restart_codex"), "Codex를 재시작해 주세요");
        m.insert((Language::Fr, "status.restart_codex"), "Veuillez redémarrer Codex");
        m.insert((Language::Es, "status.restart_codex"), "Reinicia Codex para aplicar");

        m.insert((Language::En, "status.codex_restarted"), "Codex restarted");
        m.insert((Language::ZhCn, "status.codex_restarted"), "Codex 已重启");
        m.insert((Language::ZhTw, "status.codex_restarted"), "Codex 已重啟");
        m.insert((Language::Ja, "status.codex_restarted"), "Codexを再起動しました");
        m.insert((Language::Ko, "status.codex_restarted"), "Codex 재시작됨");
        m.insert((Language::Fr, "status.codex_restarted"), "Codex redémarré");
        m.insert((Language::Es, "status.codex_restarted"), "Codex reiniciado");

        m.insert((Language::En, "status.restart_failed"), "Restart failed");
        m.insert((Language::ZhCn, "status.restart_failed"), "重启失败");
        m.insert((Language::ZhTw, "status.restart_failed"), "重啟失敗");
        m.insert((Language::Ja, "status.restart_failed"), "再起動失敗");
        m.insert((Language::Ko, "status.restart_failed"), "재시작 실패");
        m.insert((Language::Fr, "status.restart_failed"), "Échec du redémarrage");
        m.insert((Language::Es, "status.restart_failed"), "Error al reiniciar");

        m.insert((Language::En, "status.deleted"), "Deleted");
        m.insert((Language::ZhCn, "status.deleted"), "已删除");
        m.insert((Language::ZhTw, "status.deleted"), "已刪除");
        m.insert((Language::Ja, "status.deleted"), "削除しました");
        m.insert((Language::Ko, "status.deleted"), "삭제됨");
        m.insert((Language::Fr, "status.deleted"), "Supprimé");
        m.insert((Language::Es, "status.deleted"), "Eliminado");

        m.insert((Language::En, "status.delete_failed"), "Delete failed");
        m.insert((Language::ZhCn, "status.delete_failed"), "删除失败");
        m.insert((Language::ZhTw, "status.delete_failed"), "刪除失敗");
        m.insert((Language::Ja, "status.delete_failed"), "削除失敗");
        m.insert((Language::Ko, "status.delete_failed"), "삭제 실패");
        m.insert((Language::Fr, "status.delete_failed"), "Échec de la suppression");
        m.insert((Language::Es, "status.delete_failed"), "Error al eliminar");

        m.insert((Language::En, "status.renamed"), "Renamed →");
        m.insert((Language::ZhCn, "status.renamed"), "已重命名 →");
        m.insert((Language::ZhTw, "status.renamed"), "已重命名 →");
        m.insert((Language::Ja, "status.renamed"), "名前変更 →");
        m.insert((Language::Ko, "status.renamed"), "이름 변경 →");
        m.insert((Language::Fr, "status.renamed"), "Renommé →");
        m.insert((Language::Es, "status.renamed"), "Renombrado →");

        m.insert((Language::En, "status.rename_failed"), "Rename failed");
        m.insert((Language::ZhCn, "status.rename_failed"), "重命名失败");
        m.insert((Language::ZhTw, "status.rename_failed"), "重命名失敗");
        m.insert((Language::Ja, "status.rename_failed"), "名前変更失敗");
        m.insert((Language::Ko, "status.rename_failed"), "이름 변경 실패");
        m.insert((Language::Fr, "status.rename_failed"), "Échec du renommage");
        m.insert((Language::Es, "status.rename_failed"), "Error al renombrar");

        m.insert((Language::En, "status.enter_alias"), "Please enter an alias");
        m.insert((Language::ZhCn, "status.enter_alias"), "请输入别名");
        m.insert((Language::ZhTw, "status.enter_alias"), "請輸入別名");
        m.insert((Language::Ja, "status.enter_alias"), "エイリアスを入力してください");
        m.insert((Language::Ko, "status.enter_alias"), "별칭을 입력하세요");
        m.insert((Language::Fr, "status.enter_alias"), "Veuillez saisir un alias");
        m.insert((Language::Es, "status.enter_alias"), "Introduce un alias");

        m.insert((Language::En, "status.saved_live"), "Saved current live →");
        m.insert((Language::ZhCn, "status.saved_live"), "已保存当前 live →");
        m.insert((Language::ZhTw, "status.saved_live"), "已儲存目前 live →");
        m.insert((Language::Ja, "status.saved_live"), "現在のliveを保存 →");
        m.insert((Language::Ko, "status.saved_live"), "현재 live 저장됨 →");
        m.insert((Language::Fr, "status.saved_live"), "Session actuelle enregistrée →");
        m.insert((Language::Es, "status.saved_live"), "Sesión actual guardada →");

        m.insert((Language::En, "status.save_failed"), "Save failed");
        m.insert((Language::ZhCn, "status.save_failed"), "保存失败");
        m.insert((Language::ZhTw, "status.save_failed"), "儲存失敗");
        m.insert((Language::Ja, "status.save_failed"), "保存失敗");
        m.insert((Language::Ko, "status.save_failed"), "저장 실패");
        m.insert((Language::Fr, "status.save_failed"), "Échec de l'enregistrement");
        m.insert((Language::Es, "status.save_failed"), "Error al guardar");

        m.insert((Language::En, "status.enter_path"), "Please enter auth.json path");
        m.insert((Language::ZhCn, "status.enter_path"), "请输入 auth.json 路径");
        m.insert((Language::ZhTw, "status.enter_path"), "請輸入 auth.json 路徑");
        m.insert((Language::Ja, "status.enter_path"), "auth.jsonのパスを入力してください");
        m.insert((Language::Ko, "status.enter_path"), "auth.json 경로를 입력하세요");
        m.insert((Language::Fr, "status.enter_path"), "Veuillez saisir le chemin auth.json");
        m.insert((Language::Es, "status.enter_path"), "Introduce la ruta de auth.json");

        m.insert((Language::En, "status.imported"), "Imported →");
        m.insert((Language::ZhCn, "status.imported"), "已导入 →");
        m.insert((Language::ZhTw, "status.imported"), "已匯入 →");
        m.insert((Language::Ja, "status.imported"), "インポート →");
        m.insert((Language::Ko, "status.imported"), "가져오기 완료 →");
        m.insert((Language::Fr, "status.imported"), "Importé →");
        m.insert((Language::Es, "status.imported"), "Importado →");

        m.insert((Language::En, "status.import_failed"), "Import failed");
        m.insert((Language::ZhCn, "status.import_failed"), "导入失败");
        m.insert((Language::ZhTw, "status.import_failed"), "匯入失敗");
        m.insert((Language::Ja, "status.import_failed"), "インポート失敗");
        m.insert((Language::Ko, "status.import_failed"), "가져오기 실패");
        m.insert((Language::Fr, "status.import_failed"), "Échec de l'import");
        m.insert((Language::Es, "status.import_failed"), "Error al importar");

        m.insert((Language::En, "status.paste_json"), "Please paste auth.json content");
        m.insert((Language::ZhCn, "status.paste_json"), "请粘贴 auth.json 内容");
        m.insert((Language::ZhTw, "status.paste_json"), "請貼上 auth.json 內容");
        m.insert((Language::Ja, "status.paste_json"), "auth.jsonの内容を貼り付けてください");
        m.insert((Language::Ko, "status.paste_json"), "auth.json 내용을 붙여넣으세요");
        m.insert((Language::Fr, "status.paste_json"), "Veuillez coller le contenu auth.json");
        m.insert((Language::Es, "status.paste_json"), "Pega el contenido de auth.json");

        m.insert((Language::En, "status.saved_paste"), "Saved →");
        m.insert((Language::ZhCn, "status.saved_paste"), "已保存 →");
        m.insert((Language::ZhTw, "status.saved_paste"), "已儲存 →");
        m.insert((Language::Ja, "status.saved_paste"), "保存 →");
        m.insert((Language::Ko, "status.saved_paste"), "저장됨 →");
        m.insert((Language::Fr, "status.saved_paste"), "Enregistré →");
        m.insert((Language::Es, "status.saved_paste"), "Guardado →");

        m.insert((Language::En, "status.login_waiting"), "Logging in… complete authorization in browser");
        m.insert((Language::ZhCn, "status.login_waiting"), "登录中…请在浏览器完成授权");
        m.insert((Language::ZhTw, "status.login_waiting"), "登入中…請在瀏覽器完成授權");
        m.insert((Language::Ja, "status.login_waiting"), "ログイン中…ブラウザで認証を完了してください");
        m.insert((Language::Ko, "status.login_waiting"), "로그인 중… 브라우저에서 인증을 완료하세요");
        m.insert((Language::Fr, "status.login_waiting"), "Connexion… terminez l'autorisation dans le navigateur");
        m.insert((Language::Es, "status.login_waiting"), "Iniciando sesión… completa la autorización en el navegador");

        m.insert((Language::En, "status.logged_in"), "Logged in →");
        m.insert((Language::ZhCn, "status.logged_in"), "已登录 →");
        m.insert((Language::ZhTw, "status.logged_in"), "已登入 →");
        m.insert((Language::Ja, "status.logged_in"), "ログイン →");
        m.insert((Language::Ko, "status.logged_in"), "로그인됨 →");
        m.insert((Language::Fr, "status.logged_in"), "Connecté →");
        m.insert((Language::Es, "status.logged_in"), "Sesión iniciada →");

        m.insert((Language::En, "status.login_failed"), "Login failed");
        m.insert((Language::ZhCn, "status.login_failed"), "登录失败");
        m.insert((Language::ZhTw, "status.login_failed"), "登入失敗");
        m.insert((Language::Ja, "status.login_failed"), "ログイン失敗");
        m.insert((Language::Ko, "status.login_failed"), "로그인 실패");
        m.insert((Language::Fr, "status.login_failed"), "Échec de la connexion");
        m.insert((Language::Es, "status.login_failed"), "Error al iniciar sesión");

        m.insert((Language::En, "status.login_cancelled"), "Login cancelled");
        m.insert((Language::ZhCn, "status.login_cancelled"), "登录已取消");
        m.insert((Language::ZhTw, "status.login_cancelled"), "登入已取消");
        m.insert((Language::Ja, "status.login_cancelled"), "ログインをキャンセルしました");
        m.insert((Language::Ko, "status.login_cancelled"), "로그인 취소됨");
        m.insert((Language::Fr, "status.login_cancelled"), "Connexion annulée");
        m.insert((Language::Es, "status.login_cancelled"), "Inicio de sesión cancelado");

        m.insert((Language::En, "status.invalid_path"), "Path does not exist or is not a directory");
        m.insert((Language::ZhCn, "status.invalid_path"), "路径不存在或不是目录");
        m.insert((Language::ZhTw, "status.invalid_path"), "路徑不存在或不是目錄");
        m.insert((Language::Ja, "status.invalid_path"), "パスが存在しないかディレクトリではありません");
        m.insert((Language::Ko, "status.invalid_path"), "경로가 존재하지 않거나 디렉토리가 아닙니다");
        m.insert((Language::Fr, "status.invalid_path"), "Le chemin n'existe pas ou n'est pas un dossier");
        m.insert((Language::Es, "status.invalid_path"), "La ruta no existe o no es un directorio");

        m.insert((Language::En, "status.opening_dir"), "Opening directory");
        m.insert((Language::ZhCn, "status.opening_dir"), "正在打开目录");
        m.insert((Language::ZhTw, "status.opening_dir"), "正在打開目錄");
        m.insert((Language::Ja, "status.opening_dir"), "ディレクトリを開いています");
        m.insert((Language::Ko, "status.opening_dir"), "디렉토리 여는 중");
        m.insert((Language::Fr, "status.opening_dir"), "Ouverture du dossier");
        m.insert((Language::Es, "status.opening_dir"), "Abriendo directorio");

        m.insert((Language::En, "dialog.select_file"), "Select file to import");
        m.insert((Language::ZhCn, "dialog.select_file"), "选择要导入的文件");
        m.insert((Language::ZhTw, "dialog.select_file"), "選擇要匯入的檔案");
        m.insert((Language::Ja, "dialog.select_file"), "インポートするファイルを選択");
        m.insert((Language::Ko, "dialog.select_file"), "가져올 파일 선택");
        m.insert((Language::Fr, "dialog.select_file"), "Sélectionner le fichier à importer");
        m.insert((Language::Es, "dialog.select_file"), "Seleccionar archivo a importar");

        m.insert((Language::En, "status.import_cancelled"), "Import cancelled");
        m.insert((Language::ZhCn, "status.import_cancelled"), "已取消导入");
        m.insert((Language::ZhTw, "status.import_cancelled"), "已取消匯入");
        m.insert((Language::Ja, "status.import_cancelled"), "インポートをキャンセルしました");
        m.insert((Language::Ko, "status.import_cancelled"), "가져오기 취소됨");
        m.insert((Language::Fr, "status.import_cancelled"), "Import annulé");
        m.insert((Language::Es, "status.import_cancelled"), "Importación cancelada");

        m.insert((Language::En, "status.no_token"), "No OAuth token, skipping usage query");
        m.insert((Language::ZhCn, "status.no_token"), "无 OAuth token，跳过额度查询");
        m.insert((Language::ZhTw, "status.no_token"), "無 OAuth token，跳過額度查詢");
        m.insert((Language::Ja, "status.no_token"), "OAuthトークンなし、使用量クエリをスキップ");
        m.insert((Language::Ko, "status.no_token"), "OAuth 토큰 없음, 사용량 조회 건너뜀");
        m.insert((Language::Fr, "status.no_token"), "Pas de token OAuth, requête ignorée");
        m.insert((Language::Es, "status.no_token"), "Sin token OAuth, consulta omitida");

        m.insert((Language::En, "status.restart_hint"), "Edit the alias and confirm");
        m.insert((Language::ZhCn, "status.restart_hint"), "编辑别名后点确认");
        m.insert((Language::ZhTw, "status.restart_hint"), "編輯別名後點確認");
        m.insert((Language::Ja, "status.restart_hint"), "エイリアスを編集して確認");
        m.insert((Language::Ko, "status.restart_hint"), "별칭 수정 후 확인");
        m.insert((Language::Fr, "status.restart_hint"), "Modifiez l'alias puis confirmez");
        m.insert((Language::Es, "status.restart_hint"), "Edita el alias y confirma");

        m.insert((Language::En, "status.rename_cancelled"), "Rename cancelled");
        m.insert((Language::ZhCn, "status.rename_cancelled"), "已取消重命名");
        m.insert((Language::ZhTw, "status.rename_cancelled"), "已取消重命名");
        m.insert((Language::Ja, "status.rename_cancelled"), "名前変更をキャンセルしました");
        m.insert((Language::Ko, "status.rename_cancelled"), "이름 변경 취소됨");
        m.insert((Language::Fr, "status.rename_cancelled"), "Renommage annulé");
        m.insert((Language::Es, "status.rename_cancelled"), "Renombrado cancelado");

        m.insert((Language::En, "status.no_rename_pending"), "No account pending rename");
        m.insert((Language::ZhCn, "status.no_rename_pending"), "没有待重命名的账号");
        m.insert((Language::ZhTw, "status.no_rename_pending"), "沒有待重命名的賬號");
        m.insert((Language::Ja, "status.no_rename_pending"), "名前変更待ちのアカウントがありません");
        m.insert((Language::Ko, "status.no_rename_pending"), "이름 변경 대기 중인 계정 없음");
        m.insert((Language::Fr, "status.no_rename_pending"), "Aucun compte en attente de renommage");
        m.insert((Language::Es, "status.no_rename_pending"), "Ninguna cuenta pendiente de renombrar");

        m.insert((Language::En, "status.clipboard_failed"), "Failed to read clipboard");
        m.insert((Language::ZhCn, "status.clipboard_failed"), "读取剪贴板失败");
        m.insert((Language::ZhTw, "status.clipboard_failed"), "讀取剪貼板失敗");
        m.insert((Language::Ja, "status.clipboard_failed"), "クリップボードの読み取りに失敗しました");
        m.insert((Language::Ko, "status.clipboard_failed"), "클립보드 읽기 실패");
        m.insert((Language::Fr, "status.clipboard_failed"), "Échec de lecture du presse-papiers");
        m.insert((Language::Es, "status.clipboard_failed"), "Error al leer el portapapeles");

        m.insert((Language::En, "status.clipboard_empty"), "Clipboard is empty");
        m.insert((Language::ZhCn, "status.clipboard_empty"), "剪贴板为空");
        m.insert((Language::ZhTw, "status.clipboard_empty"), "剪貼板為空");
        m.insert((Language::Ja, "status.clipboard_empty"), "クリップボードが空です");
        m.insert((Language::Ko, "status.clipboard_empty"), "클립보드가 비어 있습니다");
        m.insert((Language::Fr, "status.clipboard_empty"), "Presse-papiers vide");
        m.insert((Language::Es, "status.clipboard_empty"), "Portapapeles vacío");

        m.insert((Language::En, "status.browser_login_started"), "Browser opened, please complete authorization…");
        m.insert((Language::ZhCn, "status.browser_login_started"), "已打开浏览器，请在页面完成 ChatGPT 授权…");
        m.insert((Language::ZhTw, "status.browser_login_started"), "已打開瀏覽器，請在頁面完成 ChatGPT 授權…");
        m.insert((Language::Ja, "status.browser_login_started"), "ブラウザを開きました。ChatGPT認証を完了してください…");
        m.insert((Language::Ko, "status.browser_login_started"), "브라우저를 열었습니다. ChatGPT 인증을 완료하세요…");
        m.insert((Language::Fr, "status.browser_login_started"), "Navigateur ouvert, terminez l'autorisation ChatGPT…");
        m.insert((Language::Es, "status.browser_login_started"), "Navegador abierto, completa la autorización ChatGPT…");

        m.insert((Language::En, "status.browser_login_failed"), "Browser login failed");
        m.insert((Language::ZhCn, "status.browser_login_failed"), "启动浏览器登录失败");
        m.insert((Language::ZhTw, "status.browser_login_failed"), "啟動瀏覽器登入失敗");
        m.insert((Language::Ja, "status.browser_login_failed"), "ブラウザログイン失敗");
        m.insert((Language::Ko, "status.browser_login_failed"), "브라우저 로그인 실패");
        m.insert((Language::Fr, "status.browser_login_failed"), "Échec de connexion navigateur");
        m.insert((Language::Es, "status.browser_login_failed"), "Error de inicio de sesión en navegador");

        m.insert((Language::En, "status.browser_login_ok"), "Logged in, saved as");
        m.insert((Language::ZhCn, "status.browser_login_ok"), "浏览器登录成功，已保存为");
        m.insert((Language::ZhTw, "status.browser_login_ok"), "瀏覽器登入成功，已儲存為");
        m.insert((Language::Ja, "status.browser_login_ok"), "ログイン成功、保存名：");
        m.insert((Language::Ko, "status.browser_login_ok"), "로그인 성공, 저장됨:");
        m.insert((Language::Fr, "status.browser_login_ok"), "Connecté, enregistré sous");
        m.insert((Language::Es, "status.browser_login_ok"), "Sesión iniciada, guardado como");

        m.insert((Language::En, "status.login_cancelled_by_user"), "Browser login cancelled");
        m.insert((Language::ZhCn, "status.login_cancelled_by_user"), "已取消浏览器登录");
        m.insert((Language::ZhTw, "status.login_cancelled_by_user"), "已取消瀏覽器登入");
        m.insert((Language::Ja, "status.login_cancelled_by_user"), "ブラウザログインをキャンセルしました");
        m.insert((Language::Ko, "status.login_cancelled_by_user"), "브라우저 로그인 취소됨");
        m.insert((Language::Fr, "status.login_cancelled_by_user"), "Connexion navigateur annulée");
        m.insert((Language::Es, "status.login_cancelled_by_user"), "Inicio de sesión en navegador cancelado");

        m.insert((Language::En, "status.imported_as"), "Imported");
        m.insert((Language::ZhCn, "status.imported_as"), "已导入");
        m.insert((Language::ZhTw, "status.imported_as"), "已匯入");
        m.insert((Language::Ja, "status.imported_as"), "インポートしました");
        m.insert((Language::Ko, "status.imported_as"), "가져오기 완료");
        m.insert((Language::Fr, "status.imported_as"), "Importé");
        m.insert((Language::Es, "status.imported_as"), "Importado");

        m.insert((Language::En, "status.import_text_ok"), "Imported from text as");
        m.insert((Language::ZhCn, "status.import_text_ok"), "已从文本导入为");
        m.insert((Language::ZhTw, "status.import_text_ok"), "已從文字匯入為");
        m.insert((Language::Ja, "status.import_text_ok"), "テキストからインポート：");
        m.insert((Language::Ko, "status.import_text_ok"), "텍스트에서 가져옴:");
        m.insert((Language::Fr, "status.import_text_ok"), "Importé depuis le texte sous");
        m.insert((Language::Es, "status.import_text_ok"), "Importado desde texto como");

        m.insert((Language::En, "status.import_text_failed"), "Text import failed");
        m.insert((Language::ZhCn, "status.import_text_failed"), "文本导入失败");
        m.insert((Language::ZhTw, "status.import_text_failed"), "文字匯入失敗");
        m.insert((Language::Ja, "status.import_text_failed"), "テキストインポート失敗");
        m.insert((Language::Ko, "status.import_text_failed"), "텍스트 가져오기 실패");
        m.insert((Language::Fr, "status.import_text_failed"), "Échec de l'import texte");
        m.insert((Language::Es, "status.import_text_failed"), "Error al importar texto");

        m.insert((Language::En, "status.switching_restart"), "Switched to {alias}, restarting Codex…");
        m.insert((Language::ZhCn, "status.switching_restart"), "已切换到 {alias}，正在重启 Codex…");
        m.insert((Language::ZhTw, "status.switching_restart"), "已切換到 {alias}，正在重啟 Codex…");
        m.insert((Language::Ja, "status.switching_restart"), "{alias}に切り替え、Codexを再起動中…");
        m.insert((Language::Ko, "status.switching_restart"), "{alias}로 전환됨, Codex 재시작 중…");
        m.insert((Language::Fr, "status.switching_restart"), "Basculé vers {alias}, redémarrage de Codex…");
        m.insert((Language::Es, "status.switching_restart"), "Cambiado a {alias}, reiniciando Codex…");

        m.insert((Language::En, "status.switch_ok_restart"), "Switched to {alias}");
        m.insert((Language::ZhCn, "status.switch_ok_restart"), "已切换到 {alias}");
        m.insert((Language::ZhTw, "status.switch_ok_restart"), "已切換到 {alias}");
        m.insert((Language::Ja, "status.switch_ok_restart"), "{alias}に切り替えました");
        m.insert((Language::Ko, "status.switch_ok_restart"), "{alias}로 전환됨");
        m.insert((Language::Fr, "status.switch_ok_restart"), "Basculé vers {alias}");
        m.insert((Language::Es, "status.switch_ok_restart"), "Cambiado a {alias}");

        m.insert((Language::En, "status.restart_failed_manual"), "restart failed, please restart Codex manually");
        m.insert((Language::ZhCn, "status.restart_failed_manual"), "重启失败，请手动重启 ChatGPT/Codex");
        m.insert((Language::ZhTw, "status.restart_failed_manual"), "重啟失敗，請手動重啟 ChatGPT/Codex");
        m.insert((Language::Ja, "status.restart_failed_manual"), "再起動失敗、手動で再起動してください");
        m.insert((Language::Ko, "status.restart_failed_manual"), "재시작 실패, 수동으로 재시작하세요");
        m.insert((Language::Fr, "status.restart_failed_manual"), "échec du redémarrage, redémarrez Codex manuellement");
        m.insert((Language::Es, "status.restart_failed_manual"), "error al reiniciar, reinicia Codex manualmente");

        m.insert((Language::En, "status.switch_hint"), "Switched to {alias}. Restart Codex CLI/App to take effect.");
        m.insert((Language::ZhCn, "status.switch_hint"), "已切换到 {alias}。请重启 Codex CLI/App 后生效。");
        m.insert((Language::ZhTw, "status.switch_hint"), "已切換到 {alias}。請重啟 Codex CLI/App 後生效。");
        m.insert((Language::Ja, "status.switch_hint"), "{alias}に切り替えました。Codex CLI/Appを再起動してください。");
        m.insert((Language::Ko, "status.switch_hint"), "{alias}로 전환됨. Codex CLI/App을 재시작하세요.");
        m.insert((Language::Fr, "status.switch_hint"), "Basculé vers {alias}. Redémarrez Codex CLI/App pour appliquer.");
        m.insert((Language::Es, "status.switch_hint"), "Cambiado a {alias}. Reinicia Codex CLI/App para aplicar.");

        m.insert((Language::En, "status.lang_switched"), "Language switched");
        m.insert((Language::ZhCn, "status.lang_switched"), "语言已切换");
        m.insert((Language::ZhTw, "status.lang_switched"), "語言已切換");
        m.insert((Language::Ja, "status.lang_switched"), "言語を切り替えました");
        m.insert((Language::Ko, "status.lang_switched"), "언어 변경됨");
        m.insert((Language::Fr, "status.lang_switched"), "Langue changée");
        m.insert((Language::Es, "status.lang_switched"), "Idioma cambiado");

        for (language, value) in [
            (Language::En, "Show Window"),
            (Language::ZhCn, "显示窗口"),
            (Language::ZhTw, "顯示視窗"),
            (Language::Ja, "ウィンドウを表示"),
            (Language::Ko, "창 표시"),
            (Language::Fr, "Afficher la fenêtre"),
            (Language::Es, "Mostrar ventana"),
        ] {
            m.insert((language, "tray.show"), value);
        }
        for (language, value) in [
            (Language::En, "About"),
            (Language::ZhCn, "关于"),
            (Language::ZhTw, "關於"),
            (Language::Ja, "情報"),
            (Language::Ko, "정보"),
            (Language::Fr, "À propos"),
            (Language::Es, "Acerca de"),
        ] {
            m.insert((language, "btn.about"), value);
        }
        for (language, value) in [
            (Language::En, "Restore Last"),
            (Language::ZhCn, "恢复上次删除"),
            (Language::ZhTw, "還原上次刪除"),
            (Language::Ja, "直前の削除を復元"),
            (Language::Ko, "최근 삭제 복원"),
            (Language::Fr, "Restaurer la suppression"),
            (Language::Es, "Restaurar eliminado"),
        ] {
            m.insert((language, "btn.restore"), value);
        }
        for (language, value) in [
            (Language::En, "INVALID"),
            (Language::ZhCn, "已损坏"),
            (Language::ZhTw, "已損壞"),
            (Language::Ja, "破損"),
            (Language::Ko, "손상됨"),
            (Language::Fr, "INVALIDE"),
            (Language::Es, "NO VÁLIDO"),
        ] {
            m.insert((language, "label.invalid"), value);
        }
        for (language, value) in [
            (Language::En, "Open Claude Desktop"),
            (Language::ZhCn, "打开 Claude Desktop"),
            (Language::ZhTw, "開啟 Claude Desktop"),
            (Language::Ja, "Claude Desktop を開く"),
            (Language::Ko, "Claude Desktop 열기"),
            (Language::Fr, "Ouvrir Claude Desktop"),
            (Language::Es, "Abrir Claude Desktop"),
        ] {
            m.insert((language, "claude.open"), value);
        }
        for (language, value) in [
            (Language::En, "Restored"),
            (Language::ZhCn, "已恢复"),
            (Language::ZhTw, "已還原"),
            (Language::Ja, "復元しました"),
            (Language::Ko, "복원됨"),
            (Language::Fr, "Restauré"),
            (Language::Es, "Restaurado"),
        ] {
            m.insert((language, "status.restored"), value);
        }
        for (language, value) in [
            (Language::En, "Restore failed"),
            (Language::ZhCn, "恢复失败"),
            (Language::ZhTw, "還原失敗"),
            (Language::Ja, "復元に失敗しました"),
            (Language::Ko, "복원 실패"),
            (Language::Fr, "Échec de la restauration"),
            (Language::Es, "Error al restaurar"),
        ] {
            m.insert((language, "status.restore_failed"), value);
        }
        for (language, value) in [
            (Language::En, "Claude Desktop opened; sign in, then use Save Current"),
            (Language::ZhCn, "已打开 Claude Desktop；登录后回到这里点「保存当前」"),
            (Language::ZhTw, "已開啟 Claude Desktop；登入後回到這裡點「儲存目前」"),
            (Language::Ja, "Claude Desktop を開きました。ログイン後に「現在を保存」を選んでください"),
            (Language::Ko, "Claude Desktop을 열었습니다. 로그인 후 현재 저장을 누르세요"),
            (Language::Fr, "Claude Desktop ouvert ; connectez-vous puis enregistrez le compte actuel"),
            (Language::Es, "Claude Desktop abierto; inicia sesión y guarda la cuenta actual"),
        ] {
            m.insert((language, "status.claude_opened"), value);
        }
        for (language, value) in [
            (Language::En, "Failed to open Claude Desktop"),
            (Language::ZhCn, "打开 Claude Desktop 失败"),
            (Language::ZhTw, "開啟 Claude Desktop 失敗"),
            (Language::Ja, "Claude Desktop を開けませんでした"),
            (Language::Ko, "Claude Desktop 열기 실패"),
            (Language::Fr, "Impossible d'ouvrir Claude Desktop"),
            (Language::Es, "No se pudo abrir Claude Desktop"),
        ] {
            m.insert((language, "status.claude_open_failed"), value);
        }
        for (language, value) in [
            (Language::En, "Sign in to Claude Desktop, then use Save Current"),
            (Language::ZhCn, "先登录 Claude Desktop，再使用「保存当前」"),
            (Language::ZhTw, "先登入 Claude Desktop，再使用「儲存目前」"),
            (Language::Ja, "Claude Desktop にログインしてから「現在を保存」を使用してください"),
            (Language::Ko, "Claude Desktop에 로그인한 다음 현재 저장을 사용하세요"),
            (Language::Fr, "Connectez-vous à Claude Desktop, puis enregistrez le compte actuel"),
            (Language::Es, "Inicia sesión en Claude Desktop y guarda la cuenta actual"),
        ] {
            m.insert((language, "profiles.empty_hint_claude"), value);
        }
        for (language, value) in [
            (Language::En, "Ready"),
            (Language::ZhCn, "就绪"),
            (Language::ZhTw, "就緒"),
            (Language::Ja, "準備完了"),
            (Language::Ko, "준비됨"),
            (Language::Fr, "Prêt"),
            (Language::Es, "Listo"),
        ] {
            m.insert((language, "status.ready"), value);
        }
        for (language, value) in [
            (
                Language::En,
                "Switched Claude Desktop account to {alias}; app reopened",
            ),
            (
                Language::ZhCn,
                "已切换 Claude Desktop 账号：{alias}，应用已重新打开",
            ),
            (
                Language::ZhTw,
                "已切換 Claude Desktop 帳號：{alias}，應用程式已重新開啟",
            ),
            (
                Language::Ja,
                "Claude Desktop アカウントを {alias} に切り替え、アプリを再起動しました",
            ),
            (
                Language::Ko,
                "Claude Desktop 계정을 {alias}(으)로 전환하고 앱을 다시 열었습니다",
            ),
            (
                Language::Fr,
                "Compte Claude Desktop remplacé par {alias} ; application rouverte",
            ),
            (
                Language::Es,
                "Cuenta de Claude Desktop cambiada a {alias}; aplicación reabierta",
            ),
        ] {
            m.insert((language, "status.claude_switched"), value);
        }
        for (language, value) in [
            (Language::En, "No window for this plan"),
            (Language::ZhCn, "本计划无此窗口"),
            (Language::ZhTw, "本方案無此窗口"),
            (Language::Ja, "このプランには対象枠がありません"),
            (Language::Ko, "이 요금제에는 해당 창이 없습니다"),
            (Language::Fr, "Aucune fenêtre pour cette offre"),
            (Language::Es, "Este plan no incluye esta ventana"),
        ] {
            m.insert((language, "usage.no_window"), value);
        }
        for (language, value) in [
            (Language::En, "Reset"),
            (Language::ZhCn, "重置"),
            (Language::ZhTw, "重設"),
            (Language::Ja, "リセット"),
            (Language::Ko, "초기화"),
            (Language::Fr, "Réinitialisation"),
            (Language::Es, "Reinicio"),
        ] {
            m.insert((language, "usage.reset_prefix"), value);
        }
        for (language, value) in [
            (Language::En, "Resets available"),
            (Language::ZhCn, "可用重置次数"),
            (Language::ZhTw, "可用重設次數"),
            (Language::Ja, "利用可能なリセット回数"),
            (Language::Ko, "사용 가능한 초기화 횟수"),
            (Language::Fr, "Réinitialisations disponibles"),
            (Language::Es, "Reinicios disponibles"),
        ] {
            m.insert((language, "usage.reset_credits_available"), value);
        }
        for (language, value) in [
            (Language::En, "Reset time unknown"),
            (Language::ZhCn, "重置时间未知"),
            (Language::ZhTw, "重設時間未知"),
            (Language::Ja, "リセット時刻は不明です"),
            (Language::Ko, "초기화 시간을 알 수 없습니다"),
            (Language::Fr, "Heure de réinitialisation inconnue"),
            (Language::Es, "Hora de reinicio desconocida"),
        ] {
            m.insert((language, "usage.reset_unknown"), value);
        }
        for (language, value) in [
            (Language::En, "Usage unavailable"),
            (Language::ZhCn, "额度不可用"),
            (Language::ZhTw, "額度不可用"),
            (Language::Ja, "使用量を取得できません"),
            (Language::Ko, "사용량을 확인할 수 없음"),
            (Language::Fr, "Utilisation indisponible"),
            (Language::Es, "Uso no disponible"),
        ] {
            m.insert((language, "usage.unavailable"), value);
        }
        for (language, value) in [
            (Language::En, "No window"),
            (Language::ZhCn, "无额度窗"),
            (Language::ZhTw, "無額度窗口"),
            (Language::Ja, "対象枠なし"),
            (Language::Ko, "사용량 창 없음"),
            (Language::Fr, "Aucune fenêtre"),
            (Language::Es, "Sin ventana"),
        ] {
            m.insert((language, "usage.no_window_short"), value);
        }
        for (language, value) in [
            (Language::En, "Usage pending refresh"),
            (Language::ZhCn, "额度待刷新"),
            (Language::ZhTw, "額度待刷新"),
            (Language::Ja, "使用量の更新待ち"),
            (Language::Ko, "사용량 새로고침 대기 중"),
            (Language::Fr, "Actualisation en attente"),
            (Language::Es, "Uso pendiente de actualizar"),
        ] {
            m.insert((language, "usage.pending_refresh"), value);
        }
        for (language, value) in [
            (Language::En, "Updated"),
            (Language::ZhCn, "更新"),
            (Language::ZhTw, "更新"),
            (Language::Ja, "更新"),
            (Language::Ko, "업데이트"),
            (Language::Fr, "Actualisé"),
            (Language::Es, "Actualizado"),
        ] {
            m.insert((language, "usage.updated_prefix"), value);
        }

        for (key, en, zh_cn, zh_tw, ja, ko, fr, es) in [
            (
                "detail.title",
                "ACCOUNT DETAILS",
                "账号详情",
                "帳戶詳情",
                "アカウント詳細",
                "계정 세부 정보",
                "DÉTAILS DU COMPTE",
                "DETALLES DE LA CUENTA",
            ),
            (
                "label.preview",
                "PREVIEW",
                "预览中",
                "預覽中",
                "プレビュー",
                "미리 보기",
                "APERÇU",
                "VISTA PREVIA",
            ),
            (
                "status.previewing",
                "Viewing {alias}; the active account has not changed",
                "正在查看 {alias}，使用中的账号未改变",
                "正在查看 {alias}，使用中的帳戶未改變",
                "{alias} を表示中です。使用中のアカウントは変更されていません",
                "{alias} 계정을 보는 중이며 사용 중인 계정은 변경되지 않았습니다",
                "Affichage de {alias} ; le compte actif n'a pas changé",
                "Viendo {alias}; la cuenta activa no ha cambiado",
            ),
            (
                "about.description",
                "A local GCSA utility for saving and switching Codex and Claude Desktop account profiles, viewing Codex usage, reset times, and activity insights.",
                "GCSA 内部使用的本地桌面工具，用于保存与切换 Codex、Claude Desktop 账号，并查看 Codex 额度、重置时间和活动洞察。账号资料保存在本机。",
                "GCSA 內部使用的本機桌面工具，用於儲存與切換 Codex、Claude Desktop 帳戶，並查看 Codex 額度、重設時間和活動洞察。帳戶資料儲存在本機。",
                "Codex と Claude Desktop のアカウント保存・切り替え、Codex の使用量、リセット時刻、アクティビティ確認を行う GCSA 内部向けローカルツールです。アカウント情報は端末内に保存されます。",
                "Codex 및 Claude Desktop 계정을 저장·전환하고 Codex 사용량, 재설정 시간, 활동 인사이트를 확인하는 GCSA 내부용 로컬 도구입니다. 계정 정보는 이 기기에 저장됩니다.",
                "Outil local interne à GCSA pour enregistrer et basculer les comptes Codex et Claude Desktop, puis consulter l'utilisation, les réinitialisations et l'activité Codex. Les données restent sur cet appareil.",
                "Herramienta local interna de GCSA para guardar y cambiar cuentas de Codex y Claude Desktop, y consultar uso, reinicios y actividad de Codex. Los datos de cuenta permanecen en este equipo.",
            ),
            (
                "about.third_party",
                "THIRD-PARTY SOFTWARE",
                "开源与第三方软件",
                "開源與第三方軟體",
                "オープンソース／サードパーティ",
                "오픈 소스 및 타사 소프트웨어",
                "LOGICIELS TIERS ET LIBRES",
                "SOFTWARE LIBRE Y DE TERCEROS",
            ),
            (
                "about.notices",
                "Slint attribution is required by its royalty-free license. Other dependency licenses and source URLs are included in THIRD_PARTY_NOTICES.md with the app.",
                "Slint 的免版税许可要求显示以上归属信息。其他依赖的许可证与源码 URL 收录在随应用分发的 THIRD_PARTY_NOTICES.md 中。",
                "Slint 的免版稅授權要求顯示以上歸屬資訊。其他相依套件的授權與原始碼 URL 收錄於隨應用程式散佈的 THIRD_PARTY_NOTICES.md。",
                "上記の Slint 表示はロイヤリティフリーライセンスの要件です。その他の依存関係のライセンスとソース URL は同梱の THIRD_PARTY_NOTICES.md に記載しています。",
                "위 Slint 표시는 로열티 프리 라이선스 요구 사항입니다. 기타 종속성 라이선스와 소스 URL은 앱과 함께 제공되는 THIRD_PARTY_NOTICES.md에 있습니다.",
                "L'attribution Slint ci-dessus est exigée par sa licence sans redevance. Les autres licences et URL source figurent dans le fichier THIRD_PARTY_NOTICES.md fourni avec l'application.",
                "La licencia libre de regalías de Slint exige la atribución anterior. Las demás licencias y URL del código fuente se incluyen en THIRD_PARTY_NOTICES.md junto con la aplicación.",
            ),
            (
                "about.close",
                "Close",
                "关闭",
                "關閉",
                "閉じる",
                "닫기",
                "Fermer",
                "Cerrar",
            ),
            ("activity.open", "Activity Insights", "活动洞察", "活動洞察", "アクティビティ", "활동 인사이트", "Aperçu d'activité", "Actividad"),
            ("activity.title", "Activity Insights", "活动洞察", "活動洞察", "アクティビティ分析", "활동 인사이트", "Aperçu de l'activité", "Resumen de actividad"),
            ("activity.subtitle", "Current Codex account", "当前 Codex 账户", "目前 Codex 帳戶", "現在の Codex アカウント", "현재 Codex 계정", "Compte Codex actuel", "Cuenta Codex actual"),
            ("activity.back", "Back to accounts", "返回账户", "返回帳戶", "アカウントに戻る", "계정으로 돌아가기", "Retour aux comptes", "Volver a cuentas"),
            ("activity.daily", "Daily", "每日", "每日", "日別", "일별", "Quotidien", "Diario"),
            ("activity.weekly", "Weekly", "每周", "每週", "週別", "주별", "Hebdomadaire", "Semanal"),
            ("activity.cumulative", "Cumulative", "累计", "累計", "累計", "누적", "Cumul", "Acumulado"),
            ("activity.longest_streak", "Longest streak", "最长连续天数", "最長連續天數", "最長連続日数", "최장 연속 일수", "Plus longue série", "Racha más larga"),
            ("activity.chart_title", "TOKEN ACTIVITY", "TOKEN 活动", "TOKEN 活動", "TOKEN アクティビティ", "TOKEN 활동", "ACTIVITÉ TOKEN", "ACTIVIDAD TOKEN"),
            ("activity.chart_hint", "Past 52 weeks", "过去 52 周", "過去 52 週", "過去52週間", "지난 52주", "52 dernières semaines", "Últimas 52 semanas"),
            ("activity.insights", "ACTIVITY INSIGHTS", "活动洞察", "活動洞察", "アクティビティ分析", "활동 인사이트", "APERÇU DE L'ACTIVITÉ", "RESUMEN DE ACTIVIDAD"),
            ("activity.fast_mode", "Fast mode", "快速模式", "快速模式", "高速モード", "빠른 모드", "Mode rapide", "Modo rápido"),
            ("activity.reasoning", "Most used reasoning", "最常用的推理强度", "最常用的推理強度", "最多の推論強度", "가장 많이 사용한 추론", "Raisonnement principal", "Razonamiento principal"),
            ("activity.unique_skills", "Skills explored", "已探索的技能", "已探索的技能", "探索したスキル", "탐색한 스킬", "Compétences explorées", "Habilidades exploradas"),
            ("activity.total_skills", "Total skill uses", "使用的技能总数", "技能使用總數", "スキル利用総数", "총 스킬 사용", "Utilisations de compétences", "Usos de habilidades"),
            ("activity.threads", "Total chats", "聊天总数", "聊天總數", "チャット総数", "총 채팅", "Total des discussions", "Chats totales"),
            ("activity.top", "MOST USED PLUGINS & SKILLS", "最常用的插件与技能", "最常用的外掛與技能", "よく使うプラグインとスキル", "자주 사용한 플러그인 및 스킬", "PLUGINS ET COMPÉTENCES", "PLUGINS Y HABILIDADES"),
            ("activity.runs", "runs", "次运行", "次執行", "回実行", "회 실행", "exécutions", "ejecuciones"),
            ("activity.no_data", "Refresh usage to load activity data", "刷新额度后加载活动数据", "重新整理額度後載入活動資料", "使用量を更新してデータを読み込む", "사용량을 새로고침해 활동 데이터를 불러오세요", "Actualisez l'utilisation pour charger les données", "Actualiza el uso para cargar los datos"),
        ] {
            for (language, value) in [
                (Language::En, en),
                (Language::ZhCn, zh_cn),
                (Language::ZhTw, zh_tw),
                (Language::Ja, ja),
                (Language::Ko, ko),
                (Language::Fr, fr),
                (Language::Es, es),
            ] {
                m.insert((language, key), value);
            }
        }

        // 改版导航、分析面板和明确的缓存状态。
        let redesign = [
            ("val.title", ["Token and equivalent value", "Token 与等价价值", "Token 與等價價值", "Token と換算額", "Token 및 환산 가치", "Tokens et valeur équivalente", "Tokens y valor equivalente"]),
            ("val.tokens", ["Tokens in returned dates", "已返回日期 Token 合计", "已回傳日期 Token 合計", "取得済み日付の Token 合計", "반환된 날짜 Token 합계", "Tokens des dates reçues", "Tokens de fechas recibidas"]),
            ("val.value", ["API equivalent · scenario estimate", "API 等价价值 · 假设估值", "API 等價價值 · 假設估值", "API 換算額 · 仮定の推計", "API 환산 · 가정 추정치", "Équivalent API · estimation", "Equivalente API · estimación"]),
            ("val.configure", ["Advanced ratios", "高级：估算比例", "進階：估算比例", "詳細：推定比率", "고급: 추정 비율", "Ratios avancés", "Proporciones avanzadas"]),
            ("val.model", ["Reference model / pricing scenario", "参考模型／计价方案", "參考模型／計價方案", "参考モデル・料金設定", "참조 모델 / 요금 시나리오", "Modèle / scénario tarifaire", "Modelo / escenario de precios"]),
            ("val.custom", ["Official Standard · short context", "官方标准价 · 短上下文", "官方標準價 · 短上下文", "公式標準料金 · 短文脈", "공식 표준 요금 · 짧은 문맥", "Tarif officiel Standard · contexte court", "Precio oficial Standard · contexto corto"]),
            ("val.input_ratio", ["Input share of total", "输入占总量", "輸入占總量", "入力の割合", "전체 입력 비율", "Part d’entrée", "Proporción de entrada"]),
            ("val.cache_ratio", ["Cached share of input", "缓存占输入", "快取占輸入", "入力のキャッシュ割合", "입력 중 캐시 비율", "Part en cache de l’entrée", "Proporción de entrada en caché"]),
            ("val.input_price", ["Input · USD / million", "普通输入 · USD／百万", "一般輸入 · USD／百萬", "通常入力 · USD/百万", "일반 입력 · USD/백만", "Entrée · USD / million", "Entrada · USD / millón"]),
            ("val.cache_price", ["Cached input · USD / million", "缓存输入 · USD／百万", "快取輸入 · USD／百萬", "キャッシュ入力 · USD/百万", "캐시 입력 · USD/백만", "Entrée en cache · USD / million", "Entrada en caché · USD / millón"]),
            ("val.output_price", ["Output · USD / million", "输出 · USD／百万", "輸出 · USD／百萬", "出力 · USD/百万", "출력 · USD/백만", "Sortie · USD / million", "Salida · USD / millón"]),
            ("val.price_date", ["Price snapshot verified", "价格快照核验于", "價格快照核驗於", "価格確認日", "가격 확인일", "Prix vérifiés le", "Precios verificados el"]),
            ("val.apply", ["Save and apply", "保存并应用", "儲存並套用", "保存して適用", "저장 및 적용", "Enregistrer et appliquer", "Guardar y aplicar"]),
            ("val.cancel", ["Cancel", "取消", "取消", "キャンセル", "취소", "Annuler", "Cancelar"]),
            ("val.trend", ["Daily trend", "每日趋势", "每日趨勢", "日別推移", "일별 추이", "Tendance quotidienne", "Tendencia diaria"]),
            ("val.token_mode", ["Token", "Token", "Token", "Token", "Token", "Token", "Token"]),
            ("val.cost_mode", ["Equivalent value (USD)", "等价价值（USD）", "等價價值（USD）", "換算額（USD）", "환산 가치 (USD)", "Valeur équivalente (USD)", "Valor equivalente (USD)"]),
            ("val.scope", ["Estimates returned dates only, not bills or savings; excludes cache writes, long-context, fast-mode and tool surcharges.", "按已返回日期估算，不代表账单或节省金额；未含缓存写入、长上下文、快速模式及工具等附加费用。", "依已回傳日期估算，非帳單或節省金額；不含快取寫入、長上下文、快速模式及工具等附加費用。", "取得済みの日付のみの推計。請求額ではなく、キャッシュ書込・長文・高速モード・ツール追加料金は含みません。", "반환된 날짜만 추정. 청구액이 아니며 캐시 쓰기, 긴 문맥, 빠른 모드, 도구 추가 비용 제외.", "Estimation des dates reçues, sans frais de cache, contexte long, mode rapide ni outils ; ce n’est pas une facture.", "Estimación de fechas recibidas; no es factura ni ahorro. Excluye escritura de caché, contexto largo, modo rápido y herramientas."]),
            ("val.source", ["Official pricing reference", "官方计价参考", "官方計價參考", "公式料金の参考", "공식 요금 참고", "Référence tarifaire officielle", "Referencia oficial de precios"]),
            ("val.hint", ["The account API returns total tokens only. Default scenario: 80% input, 50% cached input; output is the remainder. Adjust optionally. Prices load automatically from the official snapshot.", "账号接口只有 Token 总量。默认估算：输入占 80%，其中缓存占 50%，其余为输出；可按需调整。单价自动使用官方价格快照。", "帳號介面僅有 Token 總量。預設估算：輸入佔 80%，其中快取佔 50%，其餘為輸出；可選調整。單價自動使用官方價格快照。", "API は総 Token のみ。既定の仮定：入力80%、そのうちキャッシュ50%、残りは出力。任意で調整。単価は公式価格から自動設定。", "API는 총 Token만 반환. 기본 가정: 입력 80%, 그중 캐시 50%, 나머지 출력. 선택적으로 조정. 공식 가격 자동 적용.", "API : tokens totaux uniquement. Hypothèses : entrée 80 %, dont cache 50 %, reste en sortie. Ajustement facultatif. Tarifs officiels automatiques.", "La API solo devuelve tokens totales. Supuesto: entrada 80 %, caché 50 % de entrada; resto salida. Ajuste opcional. Precios oficiales automáticos."]),
            ("val.storage_error", ["Cannot read or save valuation.json; original file preserved.", "估值设置无法读取或保存，请检查 valuation.json；原文件保留。", "無法讀取或儲存 valuation.json；保留原檔。", "valuation.json を読み書きできません。元のファイルは保持。", "valuation.json 읽기/저장 실패. 원본 유지.", "Lecture ou écriture de valuation.json impossible ; original conservé.", "No se puede leer o guardar valuation.json; original conservado."]),
            ("val.invalid", ["Ratios must be between 0 and 100%.", "估算比例必须在 0–100% 之间。", "估算比例必須介於 0–100%。", "比率は0〜100%。", "비율은 0~100%.", "Ratios entre 0 et 100 %.", "Proporciones entre 0 y 100 %."]),
            ("val.unset", ["Set assumptions to estimate", "待设置估值假设", "待設定估值假設", "仮定の設定が必要", "추정 가정 설정 필요", "Définissez les hypothèses", "Define los supuestos"]),
            ("val.coverage", ["Returned {n}/{days} days; missing dates excluded, today may be incomplete.", "已返回 {n}/{days} 天；缺失日期不计入合计，今日可能未完整。", "已回傳 {n}/{days} 天；缺失日期不計入，今日可能未完整。", "{n}/{days} 日取得。未取得日は除外、本日は未完了の可能性。", "{n}/{days}일 반환. 누락 날짜 제외, 오늘은 불완전할 수 있음.", "{n}/{days} jours reçus ; dates absentes exclues, jour courant possiblement incomplet.", "{n}/{days} días recibidos; faltantes excluidos, hoy puede estar incompleto."]),
            ("val.missing", ["Date missing or invalid", "日期未返回或数据无效", "日期未回傳或資料無效", "日付未取得または無効", "날짜 누락 또는 무효", "Date absente ou invalide", "Fecha ausente o inválida"]),
            ("val.rates", ["Input / cache / output (USD / million)", "输入／缓存／输出单价（USD／百万）", "輸入／快取／輸出單價（USD／百萬）", "入力・キャッシュ・出力（USD/百万）", "입력 / 캐시 / 출력 (USD/백만)", "Entrée / cache / sortie (USD / million)", "Entrada / caché / salida (USD / millón)"]),

            ("new.weekly_used", ["Weekly used", "每周已用", "每週已用", "週間使用量", "주간 사용량", "Utilisation hebdomadaire", "Uso semanal"]),
            ("new.nav_accounts", ["Accounts", "账号管理", "帳號管理", "アカウント", "계정 관리", "Comptes", "Cuentas"]),
            ("new.nav_analysis", ["Analytics", "分析", "分析", "分析", "분석", "Analyse", "Análisis"]),
            ("new.network", ["Connection", "连接状态", "連線狀態", "接続状態", "연결 상태", "Connexion", "Conexión"]),
            ("new.network_hint", ["Uses the system proxy and proxy environment variables. Adjust network settings in your system, then retry.", "跟随系统代理，也支持代理环境变量。请在系统设置中调整网络后重试。", "跟隨系統代理，也支援代理環境變數。請在系統設定中調整網路後重試。", "システムプロキシと環境変数を使用します。ネットワーク設定を確認して再試行してください。", "시스템 프록시와 환경 변수를 사용합니다. 네트워크 설정 확인 후 다시 시도하세요.", "Utilise le proxy système et les variables de proxy. Vérifiez le réseau puis réessayez.", "Usa el proxy del sistema y las variables de proxy. Revise la red y reintente."]),
            ("new.add_account", ["＋ Add account", "＋ 添加账号", "＋ 新增帳號", "＋ 追加", "＋ 계정 추가", "＋ Ajouter", "＋ Añadir"]),
            ("new.cached", ["Cached", "缓存", "快取", "キャッシュ", "캐시", "En cache", "En caché"]),
            ("new.more", ["More", "更多", "更多", "その他", "더 보기", "Plus", "Más"]),
            ("new.loading", ["Updating…", "更新中…", "更新中…", "更新中…", "업데이트 중…", "Actualisation…", "Actualizando…"]),
            ("new.days", ["days", "天", "天", "日", "일", "jours", "días"]),
            ("new.plan_usage", ["Plan usage", "套餐用量", "方案用量", "プラン使用量", "플랜 사용량", "Utilisation du forfait", "Uso del plan"]),
            ("new.by_product", ["By product", "按产品", "按產品", "製品別", "제품별", "Par produit", "Por producto"]),
            ("new.by_model", ["By model", "按模型", "按模型", "モデル別", "모델별", "Par modèle", "Por modelo"]),
            ("new.by_client", ["By client", "按使用端", "按用戶端", "クライアント別", "클라이언트별", "Par client", "Por cliente"]),
            ("new.product_activity", ["Product activity", "产品活动", "產品活動", "製品の利用", "제품 활동", "Activité produit", "Actividad del producto"]),
            ("new.tool_activity", ["Tool activity", "工具活动", "工具活動", "ツールの利用", "도구 활동", "Activité des outils", "Actividad de herramientas"]),
            ("new.turns", ["Turns", "轮次", "輪次", "ターン", "턴", "Tours", "Turnos"]),
            ("new.returned_total", ["Returned items total", "已返回项目合计", "已回傳項目合計", "取得項目の合計", "반환 항목 합계", "Total des éléments reçus", "Total de elementos recibidos"]),
            ("new.plugins", ["Plugins", "插件", "外掛", "プラグイン", "플러그인", "Plugins", "Plugins"]),
            ("new.skills", ["Skills", "技能", "技能", "スキル", "스킬", "Compétences", "Habilidades"]),
            ("new.analysis_delay", ["Approximate usage; may lag by 6 hours. Daily usage is separate from the current rate-limit window.", "用量为近似值，最多可能延迟 6 小时。每日用量不等于当前窗口的已用比例。", "用量為近似值，最多可能延遲 6 小時。每日用量不等於目前視窗的已用比例。", "使用量は概算で最大6時間遅延します。日別使用量は現在の上限枠とは別です。", "사용량은 근사치이며 최대 6시간 지연될 수 있습니다. 일일 사용량은 현재 한도와 별개입니다.", "Usage approximatif, retard possible de 6 h. Les valeurs quotidiennes sont distinctes du quota actuel.", "Uso aproximado, con hasta 6 h de retraso. El uso diario es independiente de la cuota actual."]),
            ("new.analysis_source", ["Account statistics · Tool rankings aggregate returned daily items. Token activity is separate.", "官方账号统计 · 工具榜单为每日返回项目汇总；Token 活动独立展示。", "官方帳號統計 · 工具榜單為每日回傳項目彙總；Token 活動獨立顯示。", "アカウント統計 · ツールは日別取得項目の集計です。トークン利用は別に表示します。", "계정 통계 · 도구 순위는 일별 반환 항목의 합계입니다. 토큰 활동은 별도로 표시됩니다.", "Statistiques du compte · Classement des outils reçus par jour. Tokens présentés séparément.", "Estadísticas de cuenta · Herramientas devueltas por día. Los tokens se muestran por separado."]),
            ("new.cached_warning", ["Refresh failed · showing last successful data", "刷新失败 · 显示上次成功数据", "更新失敗 · 顯示上次成功資料", "更新失敗 · 前回のデータを表示", "새로 고침 실패 · 이전 데이터 표시", "Échec · dernières données disponibles", "Error · mostrando datos anteriores"]),
            ("new.last_success", ["Last success", "上次成功", "上次成功", "最終成功", "마지막 성공", "Dernier succès", "Último éxito"]),
            ("new.last_attempt", ["Last attempt", "最近尝试", "最近嘗試", "最終試行", "최근 시도", "Dernière tentative", "Último intento"]),
            ("new.unit_percent", ["Plan usage (%)", "套餐用量（%）", "方案用量（%）", "プラン使用量（%）", "플랜 사용량 (%)", "Utilisation du forfait (%)", "Uso del plan (%)"]),
            ("new.no_activity", ["No activity in this period", "该时段暂无活动", "該時段暫無活動", "この期間の利用なし", "이 기간에 활동 없음", "Aucune activité sur cette période", "Sin actividad en este período"]),
            ("new.no_data", ["Not loaded", "尚未加载", "尚未載入", "未読み込み", "아직 로드되지 않음", "Non chargé", "Sin cargar"]),
            ("new.data_updated", ["Data updated", "数据更新", "資料更新", "データ更新", "데이터 업데이트", "Données actualisées", "Datos actualizados"]),
            ("new.top_plugins", ["Daily top 8 aggregated", "每日前 8 项汇总", "每日前 8 項彙總", "日別上位8件の集計", "일별 상위 8개 합계", "Top 8 quotidien agrégé", "Top 8 diario agregado"]),
            ("new.top_skills", ["Daily top 6 aggregated", "每日前 6 项汇总", "每日前 6 項彙總", "日別上位6件の集計", "일별 상위 6개 합계", "Top 6 quotidien agrégé", "Top 6 diario agregado"]),
            ("new.other", ["Other", "其他", "其他", "その他", "기타", "Autres", "Otros"]),
            ("status.refresh_result", ["Accounts refreshed", "账号额度更新成功", "帳號額度更新成功", "更新したアカウント", "새로 고친 계정", "Comptes actualisés", "Cuentas actualizadas"]),
            ("status.activity_failed", ["Activity failures", "活动更新失败", "活動更新失敗", "利用状況の更新失敗", "활동 업데이트 실패", "Échecs d’activité", "Errores de actividad"]),
        ];
        for (key, values) in redesign {
            for (language, value) in [Language::En, Language::ZhCn, Language::ZhTw, Language::Ja, Language::Ko, Language::Fr, Language::Es].into_iter().zip(values) {
                m.insert((language, key), value);
            }
        }

        m
    })
}

pub fn t(lang: Language, key: &'static str) -> &'static str {
    strings()
        .get(&(lang, key))
        .or_else(|| strings().get(&(Language::En, key)))
        .copied()
        .unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::{strings, t, Language};

    #[test]
    fn language_codes_round_trip() {
        for language in [
            Language::En,
            Language::ZhCn,
            Language::ZhTw,
            Language::Ja,
            Language::Ko,
            Language::Fr,
            Language::Es,
        ] {
            assert_eq!(Language::from_code(language.code()), Some(language));
        }
    }

    #[test]
    fn claude_copy_is_localized_for_every_language() {
        for language in [
            Language::ZhCn,
            Language::ZhTw,
            Language::Ja,
            Language::Ko,
            Language::Fr,
            Language::Es,
        ] {
            for key in [
                "claude.session_hint",
                "claude.login_hint",
                "claude.paste_placeholder",
            ] {
                assert_ne!(t(language, key), t(Language::En, key));
            }
        }
    }

    #[test]
    fn every_english_ui_string_has_a_translation() {
        let table = strings();
        let english_keys: Vec<_> = table
            .keys()
            .filter_map(|(language, key)| (*language == Language::En).then_some(*key))
            .collect();

        for language in [
            Language::ZhCn,
            Language::ZhTw,
            Language::Ja,
            Language::Ko,
            Language::Fr,
            Language::Es,
        ] {
            for key in &english_keys {
                assert!(
                    table.contains_key(&(language, *key)),
                    "missing {language:?} translation for {key}"
                );
            }
        }
    }
}
