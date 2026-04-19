# Floem Pain Points

This file tracks places where `lazy-markdown` has to do extra work because Floem is missing a helper, has an awkward API shape, or makes an otherwise normal desktop workflow harder than it should be.

The goal is to keep these notes concrete:

- tie each point to a real flow in this repo
- record the current workaround
- explain the cost of that workaround
- describe what would make Floem easier to use

## 1. No clean public path to build a higher-level editor view from an existing `Editor`

Affected flow:
- tabbed editing in [src/workspace/editor_area.rs](../src/workspace/editor_area.rs)

What is awkward:
- Floem exposes both the low-level `Editor` type and higher-level editor helpers.
- `lazy-markdown` needs to store one `Editor` per tab and later render a view for that stored editor.
- The public API does not provide a clean way to take an existing `Editor` and wrap it in the same higher-level editor view used by Floem's editor helpers.

Current workaround:
- Use `editor_container_view(...)` directly.
- Rebuild the normal editor setup in app code.
- Wire default key handling at the lower level.

Cost:
- More app-owned editor setup code.
- More knowledge of Floem internals than a normal app should need.
- Harder to keep the app aligned with the default editor experience.

Useful upstream improvement:
- Add a supported constructor for building a higher-level editor view from an existing `Editor`.
- Or add a public helper that produces the standard editor view and style setup for a provided `Editor`.

## 2. Signal scope lifetime is easy to misuse for callback-created state

Affected flow:
- document creation in [src/workspace/documents.rs](../src/workspace/documents.rs)

What is awkward:
- Per-document state created from callbacks can outlive the reactive scope the callback happens to be running in.
- `RwSignal::new(...)` looks like a reasonable choice for long-lived state, but it can be tied to the wrong scope.
- The safer path in this app was `scope.create_rw_signal(...)`, using a stable scope owned by the app.

Current workaround:
- Always create long-lived document signals from a known stable `Scope`.

Cost:
- Easy-to-miss runtime failure mode.
- The bug only becomes obvious later, when code reads or closes the tab.
- The right choice is not obvious enough from the API surface.

Useful upstream improvement:
- Better docs around when `RwSignal::new(...)` is safe and when `scope.create_rw_signal(...)` should be preferred.
- If possible, API guidance that makes scope ownership clearer when building long-lived UI state from callbacks.

## 3. Global shortcut listeners are easy to misconfigure

Affected flow:
- app-wide keyboard routing in [src/app_keys.rs](../src/app_keys.rs)

What is awkward:
- App-wide shortcuts need to keep working when the editor is not focused.
- Floem does support fallback dispatch for shortcut-like keys, but the routing details are easy to get wrong.
- The listener API shape makes a few phase combinations look reasonable even when they do not receive the events you expect.

Current workaround:
- Use a root-level key handler with the exact event phase setup that matches Floem's fallback dispatch behavior.

Cost:
- Developers may need to read Floem event dispatch code to understand why a shortcut handler is not firing.
- The mistake is subtle because the handler can still work in some focus states.

Useful upstream improvement:
- Clearer docs and examples for app-wide shortcut handling.
- A higher-level helper for global shortcuts so app code does not need to understand fallback dispatch details.

## 4. No built-in desktop-style menu bar with keyboard control and anchored popup positioning

Affected flow:
- menu UI in [src/views/menu.rs](../src/views/menu.rs)

What is awkward:
- `lazy-markdown` needs top-level menus that open from the keyboard, move with arrow keys, and anchor popups to menu buttons.
- Floem's built-in menu helpers are fine for pointer-triggered popouts, but they do not provide a full desktop menu bar model.
- Anchored popup positioning still has to be built manually in app code.

Current workaround:
- Own the whole menu system in app code, including open state, selection state, keyboard routing, popup placement, and focus return.

Cost:
- More custom UI code than a desktop app should need for a common pattern.
- Popup placement is easier to get wrong than a built-in widget would be.
- Menu behavior becomes one of the app's larger custom subsystems.

Useful upstream improvement:
- A built-in menu bar or anchored popup menu widget with keyboard opening, arrow-key navigation, focus management, and correct overlay placement.
- Or a supported anchored overlay helper so apps do not have to rebuild popup positioning each time.

## 5. Linux context menu overlay can panic during repeated right-click interaction

Affected flow:
- editor context menu in [src/workspace/editor_area.rs](../src/workspace/editor_area.rs)

What is awkward:
- Floem exposes a `context_menu(...)` helper that looks like the direct way to add a right-click menu.
- On Linux in this app, repeated right-click interaction against the editor could panic inside Floem's view storage while its fallback context menu overlay was being updated.
- The failure was not in app-level menu item logic. It happened in Floem's own context-menu overlay and visual-change path.
- Floem's stock editor-content `PointerDown` path also checks whether the pointer device is primary instead of whether the pressed button is primary, so a mouse right-click can still trigger the normal left-click selection logic.

Current workaround:
- Do not use Floem's built-in context menu helper for the editor on Linux.
- Open an app-owned overlay menu instead and dispatch the selected action back into the stored editor.
- Keep a local copy of Floem's editor-content pointer wiring so the app can branch on the actual pointer button and place the overlay from the editor-content view's window transform.

Cost:
- The app has to own another popup surface for a standard desktop action.
- The direct toolkit helper cannot be trusted for this workflow.
- The crash is easy to hit with normal repeated right-click use.
- The app now owns a slice of editor view wiring that would otherwise be Floem's job, which raises the cost of future Floem editor updates.

Useful upstream improvement:
- Fix the lifecycle bug in Floem's Linux context menu overlay so repeated right-click open and close cycles do not leave stale view ids behind.
- Add regression coverage around repeated secondary-click interaction while a context menu is already open.
- Fix the editor-content button check to branch on `button == Secondary` rather than `pointer.is_primary_pointer()`.
## 6. Overlay focus breaks normal shell-level key routing

Affected flow:
- overlay-backed menu behavior in [src/views/menu.rs](../src/views/menu.rs)

What is awkward:
- Floem reparents `Overlay` content to the window root.
- A shell-level `KeyDown` listener attached to the normal app stack stops being an ancestor once an overlay popup takes focus.
- Visually the overlay still looks nested, so this routing change is easy to miss.

Current workaround:
- Put menu-navigation handling on the overlay root itself instead of relying on the normal app shell handler.

Cost:
- Key routing logic ends up split across more than one place.
- A design that looks correct in the normal tree can still fail once focus moves into an overlay.

Useful upstream improvement:
- Clear docs that focused overlay content is no longer under normal app-stack ancestors for key routing.
- A supported window-level key handling pattern that keeps working across both normal views and overlay views.

## 7. Editor focus target is not immediately available when app state switches documents

Affected flow:
- tab activation and focus in [src/workspace/documents.rs](../src/workspace/documents.rs) and [src/workspace/editor_area.rs](../src/workspace/editor_area.rs)

What is awkward:
- The app can mark a document active immediately.
- The matching editor focus target only exists later, after the editor view has been built and assigned a view id.
- That means "activate this document" and "focus its editor" are not one safe immediate operation.

Current workaround:
- Sync focus in the rendered editor view, where both the active-document state and the assigned editor view id are visible.

Cost:
- Focus code has to live in the view layer rather than near the state transition.
- App code cannot treat activation and focus as one direct action.

Useful upstream improvement:
- A clearer supported pattern for focusing a stored `Editor` when its view appears.
- Or a built-in way for an editor view to follow an app-owned active state without manual reactive focus wiring.

## 8. App-owned custom surfaces cannot reuse Floem theme data cleanly

Affected flow:
- app theme code in [src/preferences/theme.rs](../src/preferences/theme.rs)
- custom chrome in [src/main.rs](../src/main.rs), [src/views/menu.rs](../src/views/menu.rs), [src/workspace/tabs.rs](../src/workspace/tabs.rs), and [src/views/dialogs.rs](../src/views/dialogs.rs)

What is awkward:
- Floem's built-in light and dark support works well for standard widgets and built-in classes.
- App-owned surfaces still need their own styling.
- There is no small public path for app code to extend the active theme with app-defined values and reuse them through one shared theme system.

Current workaround:
- Keep a separate app-owned theme layer and map it to Floem's window theme changes.

Cost:
- Theme values for custom surfaces have to be managed separately.
- More duplicated theme work.
- Harder to keep custom app chrome aligned with built-in widgets.

Useful upstream improvement:
- A supported way for app code to extend the active theme with custom color and spacing values.
- Or a cleaner custom-element theme hook that updates with the same light, dark, and follow-OS flow.

## 9. Dropdown is not good enough for a desktop-style font picker

Affected flow:
- font selector in [src/views/menu.rs](../src/views/menu.rs)

What is awkward:
- Floem's `Dropdown` did not support keyboard navigation in the way a desktop font picker needs.
- It also did not track pointer movement by updating the highlighted option as the mouse moved across the list.
- Popup sizing and overlay positioning are still mostly internal to the widget.
- App code does not get a clean public way to cap popup height or control the visible scroll area for large option lists.

Current workaround:
- Replace the old dropdown-based font picker with the same app-owned menu system used for the rest of the menu bar.

Cost:
- More app-owned menu code for something that should have been close to a built-in widget.
- The old dropdown was not acceptable for keyboard-first use.
- The old dropdown also gave weaker pointer feedback than a normal desktop menu.

Useful upstream improvement:
- Support keyboard navigation and hover-driven highlight updates in `Dropdown`.
- Expose a supported way to configure dropdown popup sizing, especially max height.
- Or expose the popup container and scroll styling hooks needed to make long dropdowns behave like normal desktop pickers.

## 10. Editor content has no built-in inset to stay clear of overlaid scrollbars

Affected flow:
- editor area in [src/workspace/editor_area.rs](../src/workspace/editor_area.rs)

What is awkward:
- Floem's editor content is wrapped in a `Scroll` view whose vertical scrollbar is drawn over the same visual area as the text.
- Floem exposes scrollbar styling and track inset controls, but not a small editor-content inset for reserving space so text does not sit under the vertical bar.
- Floem's default `editor_content` helper styles the editor view with `position: absolute`. Absolute children use the parent's padding box as their containing block, so the obvious fix of wrapping the editor in a padded container (or adding padding to the `Scroll`) does not shrink the editor's paint area at all.
- Removing `position: absolute` is also not free: an earlier attempt that kept `absolute()` and wrapped the editor in a `Container` sized to `size_pct(100%, 100%)` with `padding_right` caused the `Scroll` view to read the wrapper's layout rect as the content size, see exactly the viewport size, and stop showing a scrollbar at all. Both pieces of this problem (absolute positioning and overflow detection through a wrapper) have to be solved together.

Current workaround:
- Drop `position: absolute` from the editor content view so it participates in normal flow and respects the scroll view's content box.
- Apply `padding_right` on the `Scroll` itself to reserve space for the bar. Because the child is no longer absolute, the padding actually shrinks the text area instead of being ignored.

Cost:
- App code owns a slice of what is normally Floem's default editor wiring, which raises the cost of future Floem editor updates.
- The fix has to diverge from Floem's own `editor_content` helper even though the rest of that helper's structure is still desirable.

Useful upstream improvement:
- Add a supported editor-content inset or right padding hook for the scrollable text region.
- Or make the default editor scrollbars reserve layout space instead of overlapping the content.
- Or drop `position: absolute` from the default editor content view so normal container padding works as expected.
