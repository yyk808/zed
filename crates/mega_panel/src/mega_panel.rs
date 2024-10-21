use std::sync::Arc;
use anyhow::{anyhow, Context};
use db::kvp::KEY_VALUE_STORE;
use file_icons::FileIcons;
use fs::Fs;
use gpui::{actions, Action, AppContext, AssetSource, AsyncWindowContext, Entity, EventEmitter, FocusHandle, FocusableView, IntoElement, Model, Pixels, Render, Subscription, Task, UniformListScrollHandle, View, ViewContext, VisualContext, WeakModel, WeakView, WindowContext};
use gpui::private::serde_derive::{Deserialize, Serialize};
use gpui::private::serde_json;
use mega::{Mega, MegaFuse};
use settings::{Settings, SettingsStore};
use util::{ResultExt, TryFutureExt};
use workspace::dock::{DockPosition, Panel, PanelEvent, PanelId};
use workspace::ui::{v_flex, IconName};
use workspace::{Pane, Workspace};
use crate::mega_panel_settings::{MegaPanelDockPosition, MegaPanelSettings};

mod mega_panel_settings;

const MEGA_PANEL_KEY: &str = "MegaPanel";

actions!(
    mega_panel,
    [
        Open,
        ToggleFocus,
        ToggleFuseMount,
    ]
);

pub struct MegaPanel {
    mega: WeakModel<Mega>,
    workspace: WeakView<Workspace>,
    focus_handle: FocusHandle,
    fs: Arc<dyn Fs>,
    pending_serialization: Task<Option<()>>,
    width: Option<Pixels>,
}

#[derive(Serialize, Deserialize)]
struct SerializedMegaPanel {
    width: Option<Pixels>,
}

#[derive(Debug)]
pub enum Event {
    Focus,
}

pub fn init_settings(cx: &mut AppContext) {
    MegaPanelSettings::register(cx);
}

pub fn init(assets: impl AssetSource, cx: &mut AppContext) {
    init_settings(cx);
    println!("Mega settings should be registered");
    file_icons::init(assets, cx);

    cx.observe_new_views(|workspace: &mut Workspace, _| {
        workspace.register_action(|workspace, _: &ToggleFocus, cx| {
            workspace.toggle_panel_focus::<MegaPanel>(cx);
        });
    })
        .detach();
}

impl EventEmitter<Event> for MegaPanel {}

impl EventEmitter<PanelEvent> for MegaPanel {}

impl Render for MegaPanel {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
    }
}

impl FocusableView for MegaPanel {
    fn focus_handle(&self, _cx: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for MegaPanel {
    fn persistent_name() -> &'static str {
        "Mega Panel"
    }

    fn position(&self, cx: &WindowContext) -> DockPosition {
        match MegaPanelSettings::get_global(cx).dock {
            MegaPanelDockPosition::Left => DockPosition::Left,
            MegaPanelDockPosition::Right => DockPosition::Right,
        }
    }

    fn position_is_valid(&self, position: DockPosition) -> bool {
        matches!(position, DockPosition::Left | DockPosition::Right)
    }

    fn set_position(&mut self, position: DockPosition, cx: &mut ViewContext<Self>) {
        settings::update_settings_file::<MegaPanelSettings>(
            self.fs.clone(),
            cx,
            move |settings, _| {
                let dock = match position {
                    DockPosition::Left | DockPosition::Bottom => MegaPanelDockPosition::Left,
                    DockPosition::Right => MegaPanelDockPosition::Right,
                };
                settings.dock = Some(dock);
            },
        );
    }

    fn size(&self, cx: &WindowContext) -> Pixels {
        self.width
            .unwrap_or_else(|| MegaPanelSettings::get_global(cx).default_width)
    }

    fn set_size(&mut self, size: Option<Pixels>, cx: &mut ViewContext<Self>) {
        self.width = size;
        self.serialize(cx);
        cx.notify();
    }

    fn icon(&self, cx: &WindowContext) -> Option<IconName> {
        MegaPanelSettings::get_global(cx)
            .button
            .then_some(IconName::FileGit)
    }

    fn icon_tooltip(&self, cx: &WindowContext) -> Option<&'static str> {
        Some("Mega Panel")
    }

    fn toggle_action(&self) -> Box<dyn Action> {
        Box::new(ToggleFocus)
    }
}

impl MegaPanel {
    pub async fn load(
        workspace: WeakView<Workspace>,
        mut cx: AsyncWindowContext,
    ) -> anyhow::Result<View<Self>> {
        let serialized_panel = cx
            .background_executor()
            .spawn(async move { KEY_VALUE_STORE.read_kvp(MEGA_PANEL_KEY) })
            .await
            .map_err(|e| anyhow!("Failed to load mega panel: {}", e))
            .context("loading mega panel")
            .log_err()
            .flatten()
            .map(|panel| serde_json::from_str::<SerializedMegaPanel>(&panel))
            .transpose()
            .log_err()
            .flatten();

        workspace.update(
            &mut cx,
            |workspace, cx| {
                let panel = MegaPanel::new(workspace, cx);
                if let Some(serialized_panel) = serialized_panel {
                    panel.update(cx, |panel, cx| {
                        panel.width = serialized_panel.width.map(|px| px.round());
                        cx.notify();
                    });
                }
                panel
            }
        )
    }

    fn new(workspace: &mut Workspace, cx: &mut ViewContext<Workspace>) -> View<Self> {
        let mega_panel = cx.new_view(|cx| {
            let mega = workspace.mega();
            
            let focus_handle = cx.focus_handle();
            cx.on_focus(&focus_handle, Self::focus_in).detach();
            
            cx.subscribe(mega, |this, mega, event, cx| {
                // TODO: listen for user operations
            }).detach();
            
            Self {
                mega: mega.downgrade(),
                workspace: workspace.weak_handle(),
                focus_handle,
                fs: workspace.app_state().fs.clone(),
                pending_serialization: Task::ready(None),
                width: None,
            }
        });

        mega_panel
    }

    fn serialize(&mut self, cx: &mut ViewContext<Self>) {
        let width = self.width;
        self.pending_serialization = cx.background_executor().spawn(
            async move {
                KEY_VALUE_STORE
                    .write_kvp(
                        MEGA_PANEL_KEY.into(),
                        serde_json::to_string(&SerializedMegaPanel { width })?,
                    )
                    .await?;
                anyhow::Ok(())
            }
                .log_err(),
        );
    }

    fn focus_in(&mut self, cx: &mut ViewContext<Self>) {
        if !self.focus_handle.contains_focused(cx) {
            cx.emit(Event::Focus);
        }
    }
}