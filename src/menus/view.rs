use floem::AnyView;
use floem::{
    prelude::{SignalGet, *},
    reactive::Effect,
    view::ViewId,
    views::{Container, Empty, Label, Overlay, Stack, dyn_stack},
};

use crate::{
    commands::CommandRegistry,
    menus::{
        app_menu_models, handle_open_menu_key_down,
        keys::toggle_menu,
        model::{AppMenuEntry, AppMenuModel, PopupRow},
    },
    workspace::{AppState, TopLevelMenuId},
};

fn popup_row_view(menu_id: TopLevelMenuId, row: PopupRow, state: AppState) -> AnyView {
    match row.entry {
        AppMenuEntry::Separator => Empty::new()
            .style({
                let state = state.clone();
                move |s| {
                    let theme = state.app_theme();
                    s.height(1.0).margin_vert(5.0).background(theme.border)
                }
            })
            .into_any(),
        AppMenuEntry::Item(item) => {
            let enabled = item.enabled;
            let action = item.action.clone();
            let click_state = state;
            let hover_state = click_state.clone();
            let style_state = click_state.clone();

            Label::new(item.title)
                .style(move |s| {
                    let theme = style_state.app_theme();
                    let menu_state = style_state.menu_state.get();
                    let selected = menu_state.open_menu == Some(menu_id)
                        && menu_state.selected_index == row.index;
                    s.width_full()
                        .selectable(false)
                        .font_size(14.0)
                        .padding_horiz(12.0)
                        .padding_vert(7.0)
                        .color(if enabled {
                            theme.text
                        } else {
                            theme.text_muted
                        })
                        .background(if selected && enabled {
                            theme.menu_popup_selected_bg
                        } else {
                            theme.menu_popup_bg
                        })
                })
                .on_event_stop(listener::PointerEnter, move |_, _| {
                    if enabled {
                        crate::menus::select_menu_index(&hover_state, menu_id, row.index);
                    }
                })
                .on_event_stop(listener::Click, move |_, _| {
                    let Some(action) = action.clone() else {
                        return;
                    };

                    if enabled {
                        crate::menus::execute_menu_action(&click_state, action);
                    }
                })
                .into_any()
        }
    }
}

fn popup_content_view(
    menu_model: AppMenuModel,
    anchor_id: ViewId,
    command_registry: CommandRegistry,
    state: AppState,
) -> impl IntoView {
    let popup_rows_state = state.clone();
    let popup_style_state = state.clone();
    let popup_row_view_state = state.clone();
    let popup_registry = command_registry.clone();
    let popup_menu_id = menu_model.id;

    Container::derived(move || {
        Stack::vertical_from_iter(
            crate::menus::popup_rows(popup_menu_id, &popup_registry, &popup_rows_state)
                .into_iter()
                .map(|row| popup_row_view(popup_menu_id, row, popup_row_view_state.clone())),
        )
        .style(|s| s.row_gap(2.0).width_full())
    })
    .style(move |s| {
        let theme = popup_style_state.app_theme();
        let is_active = popup_style_state.menu_state.get().open_menu == Some(menu_model.id);
        let anchor_rect = anchor_id.get_visual_rect();
        let inset_left = anchor_rect.x0.round();
        let inset_top = (anchor_rect.y1 + 4.0).round();
        s.fixed()
            .inset_top(inset_top)
            .inset_left(inset_left)
            .min_width(260.0)
            .padding(6.0)
            .border(1.0)
            .border_color(theme.border)
            .background(theme.menu_popup_bg)
            .z_index(200)
            .apply_if(!is_active, |s| s.hide())
    })
    .on_event_stop(listener::Click, move |_, _| {})
}

fn popup_overlay(
    menu_model: AppMenuModel,
    anchor_id: ViewId,
    popup_id: ViewId,
    command_registry: CommandRegistry,
    state: AppState,
) -> Overlay {
    let popup_child_state = state.clone();
    let popup_key_state = state;
    let popup_child_registry = command_registry.clone();
    let popup_key_registry = command_registry;

    Effect::new({
        let popup_focus_state = popup_child_state.clone();
        move |_| {
            if popup_focus_state.menu_state.get().open_menu == Some(menu_model.id) {
                popup_id.request_focus();
            }
        }
    });

    Overlay::with_id(popup_id)
        .derived_child(move || {
            popup_content_view(
                menu_model.clone(),
                anchor_id,
                popup_child_registry.clone(),
                popup_child_state.clone(),
            )
        })
        .style(|s| s.keyboard_navigable())
        .on_event_stop(listener::KeyDown, {
            move |_, event| {
                handle_open_menu_key_down(&popup_key_state, &popup_key_registry, event);
            }
        })
}

fn menu_button(
    menu_model: AppMenuModel,
    command_registry: CommandRegistry,
    state: AppState,
) -> impl IntoView {
    let anchor_id = ViewId::new();
    let popup_id = ViewId::new();
    state.register_menu_popup(menu_model.id, popup_id);
    popup_id.set_style_parent(anchor_id);
    let button_state = state.clone();
    let button_registry = command_registry.clone();
    let popup = popup_overlay(
        menu_model.clone(),
        anchor_id,
        popup_id,
        command_registry,
        state.clone(),
    );
    let anchor = Container::with_id(
        anchor_id,
        Label::new(menu_model.title.clone())
            .style(move |s| {
                let theme = state.app_theme();
                let is_active = state.menu_state.get().open_menu == Some(menu_model.id);
                s.selectable(false)
                    .padding_horiz(6.0)
                    .padding_vert(3.0)
                    .border(1.0)
                    .border_color(if is_active {
                        theme.menu_button_border_active
                    } else {
                        theme.menu_button_border
                    })
                    .border_radius(4.0)
                    .background(if is_active {
                        theme.menu_button_bg_active
                    } else {
                        theme.menu_button_bg
                    })
                    .color(theme.text)
                    .hover(|s| s.background(theme.menu_button_bg_hover))
                    .active(|s| s.background(theme.menu_button_bg_pressed))
            })
            .on_event_stop(listener::Click, move |_, _| {
                toggle_menu(&button_state, menu_model.id, &button_registry);
            }),
    );

    Stack::new((anchor, popup))
}

pub(crate) fn menu_bar_view(command_registry: CommandRegistry, state: AppState) -> impl IntoView {
    let menu_state = state.clone();
    let model_registry = command_registry.clone();
    let button_registry = command_registry;
    dyn_stack(
        move || app_menu_models(&model_registry, &menu_state),
        |menu| menu.id,
        move |menu| menu_button(menu, button_registry.clone(), state.clone()),
    )
    .style(|s| s.flex_row().col_gap(6.0))
    .on_event_stop(listener::Click, move |_, _| {})
}
