//! API-key authentication step UI and state transitions used by onboarding.

#![allow(clippy::unwrap_used)]

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;
use thinwedge_app_server_client::AppServerRequestHandle;
use thinwedge_app_server_protocol::AccountLoginCompletedNotification;
use thinwedge_app_server_protocol::AccountUpdatedNotification;
use thinwedge_app_server_protocol::AuthMode as AppServerAuthMode;
use thinwedge_app_server_protocol::ClientRequest;
use thinwedge_app_server_protocol::LoginAccountParams;
use thinwedge_app_server_protocol::LoginAccountResponse;
use thinwedge_login::read_preferred_api_key_env_var_name;
use thinwedge_login::read_preferred_api_key_from_env;

use std::cell::Cell;
use std::sync::Arc;
use std::sync::RwLock;
use thinwedge_protocol::config_types::ForcedLoginMethod;
use uuid::Uuid;

use crate::LoginStatus;
use crate::key_hint::KeyBinding;
use crate::key_hint::KeyBindingListExt;
use crate::onboarding::keys;
use crate::onboarding::onboarding_screen::KeyboardHandler;
use crate::onboarding::onboarding_screen::StepStateProvider;
use crate::tui::FrameRequester;

use super::onboarding_screen::StepState;

#[derive(Clone)]
pub(crate) enum SignInState {
    PickMode,
    ApiKeyEntry(ApiKeyInputState),
    ApiKeyConfigured,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SignInOption {
    ApiKey,
}

fn onboarding_request_id() -> thinwedge_app_server_protocol::RequestId {
    thinwedge_app_server_protocol::RequestId::String(Uuid::new_v4().to_string())
}

pub(crate) fn mark_url_hyperlink(_buf: &mut Buffer, _area: Rect, _url: &str) {}

#[derive(Clone, Default)]
pub(crate) struct ApiKeyInputState {
    value: String,
    prepopulated_from_env: bool,
    env_var_name: Option<&'static str>,
}

#[derive(Clone)]
pub(crate) struct AuthModeWidget {
    pub request_frame: FrameRequester,
    pub highlighted_mode: SignInOption,
    pub error: Arc<RwLock<Option<String>>>,
    pub sign_in_state: Arc<RwLock<SignInState>>,
    pub login_status: LoginStatus,
    pub app_server_request_handle: AppServerRequestHandle,
    pub forced_login_method: Option<ForcedLoginMethod>,
    pub animations_enabled: bool,
    pub animations_suppressed: Cell<bool>,
}

impl AuthModeWidget {
    pub(crate) fn should_suppress_animations(&self) -> bool {
        false
    }

    pub(crate) fn set_animations_suppressed(&self, suppressed: bool) {
        self.animations_suppressed.set(suppressed);
    }

    pub(crate) fn cancel_active_attempt(&self) {}

    fn set_error(&self, message: Option<String>) {
        *self.error.write().unwrap() = message;
    }

    fn error_message(&self) -> Option<String> {
        self.error.read().unwrap().clone()
    }

    pub(crate) fn is_api_key_entry_active(&self) -> bool {
        self.sign_in_state
            .read()
            .is_ok_and(|guard| matches!(&*guard, SignInState::ApiKeyEntry(_)))
    }

    pub(crate) fn api_key_entry_has_text(&self) -> bool {
        self.sign_in_state.read().is_ok_and(
            |guard| matches!(&*guard, SignInState::ApiKeyEntry(state) if !state.value.is_empty()),
        )
    }

    fn confirm_binding(&self) -> KeyBinding {
        keys::CONFIRM[0]
    }

    fn cancel_binding(&self) -> KeyBinding {
        keys::CANCEL[0]
    }

    fn start_api_key_entry(&mut self) {
        self.set_error(None);
        let prefill = read_preferred_api_key_from_env();
        let env_var_name = read_preferred_api_key_env_var_name();
        let mut guard = self.sign_in_state.write().unwrap();
        match &mut *guard {
            SignInState::ApiKeyEntry(state) => {
                if state.value.is_empty() {
                    state.value = prefill.clone().unwrap_or_default();
                    state.prepopulated_from_env = prefill.is_some();
                    state.env_var_name = env_var_name;
                }
            }
            _ => {
                *guard = SignInState::ApiKeyEntry(ApiKeyInputState {
                    value: prefill.clone().unwrap_or_default(),
                    prepopulated_from_env: prefill.is_some(),
                    env_var_name,
                });
            }
        }
        drop(guard);
        self.request_frame.schedule_frame();
    }

    fn save_api_key(&mut self, api_key: String) {
        self.set_error(None);
        let request_handle = self.app_server_request_handle.clone();
        let sign_in_state = self.sign_in_state.clone();
        let error = self.error.clone();
        let request_frame = self.request_frame.clone();
        tokio::spawn(async move {
            match request_handle
                .request_typed::<LoginAccountResponse>(ClientRequest::LoginAccount {
                    request_id: onboarding_request_id(),
                    params: LoginAccountParams::ApiKey {
                        api_key: api_key.clone(),
                    },
                })
                .await
            {
                Ok(LoginAccountResponse::ApiKey {}) => {
                    *error.write().unwrap() = None;
                    *sign_in_state.write().unwrap() = SignInState::ApiKeyConfigured;
                }
                Err(err) => {
                    *error.write().unwrap() = Some(format!("Failed to save API key: {err}"));
                    *sign_in_state.write().unwrap() = SignInState::ApiKeyEntry(ApiKeyInputState {
                        value: api_key,
                        prepopulated_from_env: false,
                        env_var_name: None,
                    });
                }
            }
            request_frame.schedule_frame();
        });
        self.request_frame.schedule_frame();
    }

    pub(crate) fn on_account_login_completed(
        &mut self,
        _notification: AccountLoginCompletedNotification,
    ) {
    }

    pub(crate) fn on_account_updated(&mut self, notification: AccountUpdatedNotification) {
        self.login_status = notification
            .auth_mode
            .map(LoginStatus::AuthMode)
            .unwrap_or(LoginStatus::NotAuthenticated);
    }

    fn render_pick_mode(&self, area: Rect, buf: &mut Buffer) {
        let mut lines: Vec<Line> = vec![
            Line::from(vec![
                "  ".into(),
                "ThinWedge uses API-key authentication.".into(),
            ]),
            Line::from(vec![
                "  ".into(),
                "Connect an OpenRouter-compatible API key to continue.".into(),
            ]),
            "".into(),
            Line::from(vec!["> 1. ".cyan().dim(), "Provide your API key".cyan()]),
            "     Works with OpenRouter and other compatible providers"
                .dim()
                .cyan()
                .into(),
            "".into(),
            Line::from(vec![
                "  Press ".dim(),
                self.confirm_binding().into(),
                " to continue".dim(),
            ]),
        ];
        if let Some(err) = self.error_message() {
            lines.push("".into());
            lines.push(err.red().into());
        }

        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn render_api_key_configured(&self, area: Rect, buf: &mut Buffer) {
        let lines = vec![
            "✓ API key configured".green().into(),
            "".into(),
            "  ThinWedge will use your configured API key for requests.".into(),
        ];

        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }

    fn render_api_key_entry(&self, area: Rect, buf: &mut Buffer, state: &ApiKeyInputState) {
        let [intro_area, input_area, footer_area] = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .areas(area);

        let mut intro_lines: Vec<Line> = vec![
            Line::from(vec![
                "> ".into(),
                "Provide your API key for ThinWedge".bold(),
            ]),
            "".into(),
            "  Paste or type your API key below. It will be stored locally in auth.json.".into(),
            "".into(),
        ];
        if state.prepopulated_from_env {
            let env_var_name = state.env_var_name.unwrap_or("OPENROUTER_API_KEY");
            intro_lines.push(format!("  Detected {env_var_name}.").into());
            intro_lines.push(
                "  Paste a different key if you prefer to use another account."
                    .dim()
                    .into(),
            );
            intro_lines.push("".into());
        }
        Paragraph::new(intro_lines)
            .wrap(Wrap { trim: false })
            .render(intro_area, buf);

        let content_line: Line = if state.value.is_empty() {
            vec!["Paste or type your API key".dim()].into()
        } else {
            Line::from(state.value.clone())
        };
        Paragraph::new(content_line)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title("API key")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .render(input_area, buf);

        let mut footer_lines: Vec<Line> = vec![
            Line::from(vec![
                "  Press ".dim(),
                self.confirm_binding().into(),
                " to save".dim(),
            ]),
            Line::from(vec![
                "  Press ".dim(),
                self.cancel_binding().into(),
                " to go back".dim(),
            ]),
        ];
        if let Some(error) = self.error_message() {
            footer_lines.push("".into());
            footer_lines.push(error.red().into());
        }
        Paragraph::new(footer_lines)
            .wrap(Wrap { trim: false })
            .render(footer_area, buf);
    }

    fn handle_api_key_entry_key_event(&mut self, key_event: &KeyEvent) -> bool {
        let mut should_save: Option<String> = None;
        let mut should_request_frame = false;

        {
            let mut guard = self.sign_in_state.write().unwrap();
            if let SignInState::ApiKeyEntry(state) = &mut *guard {
                if keys::CANCEL.is_pressed(*key_event) {
                    *guard = SignInState::PickMode;
                    self.set_error(None);
                    should_request_frame = true;
                } else if keys::CONFIRM.is_pressed(*key_event) {
                    let trimmed = state.value.trim().to_string();
                    if trimmed.is_empty() {
                        self.set_error(Some("API key cannot be empty".to_string()));
                        should_request_frame = true;
                    } else {
                        should_save = Some(trimmed);
                    }
                } else {
                    match key_event.code {
                        KeyCode::Backspace => {
                            if state.prepopulated_from_env {
                                state.value.clear();
                                state.prepopulated_from_env = false;
                                state.env_var_name = None;
                            } else {
                                state.value.pop();
                            }
                            self.set_error(None);
                            should_request_frame = true;
                        }
                        KeyCode::Char(c)
                            if key_event.kind == KeyEventKind::Press
                                && !key_event.modifiers.contains(KeyModifiers::SUPER)
                                && !key_event.modifiers.contains(KeyModifiers::CONTROL)
                                && !key_event.modifiers.contains(KeyModifiers::ALT) =>
                        {
                            if state.prepopulated_from_env {
                                state.value.clear();
                                state.prepopulated_from_env = false;
                                state.env_var_name = None;
                            }
                            state.value.push(c);
                            self.set_error(None);
                            should_request_frame = true;
                        }
                        _ => {}
                    }
                }
            } else {
                return false;
            }
        }

        if let Some(api_key) = should_save {
            self.save_api_key(api_key);
        } else if should_request_frame {
            self.request_frame.schedule_frame();
        }
        true
    }

    fn handle_api_key_entry_paste(&mut self, pasted: String) -> bool {
        let trimmed = pasted.trim();
        if trimmed.is_empty() {
            return false;
        }

        let mut guard = self.sign_in_state.write().unwrap();
        if let SignInState::ApiKeyEntry(state) = &mut *guard {
            if state.prepopulated_from_env {
                state.value = trimmed.to_string();
                state.prepopulated_from_env = false;
                state.env_var_name = None;
            } else {
                state.value.push_str(trimmed);
            }
            self.set_error(None);
        } else {
            return false;
        }

        drop(guard);
        self.request_frame.schedule_frame();
        true
    }
}

impl KeyboardHandler for AuthModeWidget {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.handle_api_key_entry_key_event(&key_event) {
            return;
        }

        if keys::CONFIRM.is_pressed(key_event) {
            if matches!(&*self.sign_in_state.read().unwrap(), SignInState::PickMode) {
                self.start_api_key_entry();
            }
            return;
        }
        if keys::CANCEL.is_pressed(key_event) {
            self.set_error(None);
        }
    }

    fn handle_paste(&mut self, pasted: String) {
        let _ = self.handle_api_key_entry_paste(pasted);
    }
}

impl StepStateProvider for AuthModeWidget {
    fn get_step_state(&self) -> StepState {
        match &*self.sign_in_state.read().unwrap() {
            SignInState::PickMode | SignInState::ApiKeyEntry(_) => StepState::InProgress,
            SignInState::ApiKeyConfigured => StepState::Complete,
        }
    }
}

impl WidgetRef for AuthModeWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        match &*self.sign_in_state.read().unwrap() {
            SignInState::PickMode => self.render_pick_mode(area, buf),
            SignInState::ApiKeyEntry(state) => self.render_api_key_entry(area, buf, state),
            SignInState::ApiKeyConfigured => self.render_api_key_configured(area, buf),
        }
    }
}

impl Widget for AuthModeWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_ref(area, buf);
    }
}

#[allow(dead_code)]
fn _app_server_auth_mode_is_api_only(auth_mode: AppServerAuthMode) -> bool {
    matches!(auth_mode, AppServerAuthMode::ApiKey)
}
