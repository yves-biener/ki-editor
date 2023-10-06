/// NOTE: all test cases that involves the clipboard should not be run in parallel
///   otherwise the the test suite will fail because multiple tests are trying to
///   access the clipboard at the same time.
#[cfg(test)]
mod test_app {
    use my_proc_macros::key;
    use serial_test::serial;

    use std::sync::{Arc, Mutex};
    use DispatchEditor::*;

    use shared::canonicalized_path::CanonicalizedPath;

    use crate::{
        app::{App, Dispatch},
        components::editor::{Direction, DispatchEditor, Movement},
        frontend::mock::MockFrontend,
        integration_test::integration_test::TestRunner,
        lsp::{process::LspNotification, signature_help::SignatureInformation},
        selection::SelectionMode,
    };

    fn run_test(
        callback: impl Fn(App<MockFrontend>, CanonicalizedPath) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        TestRunner::run(|temp_dir| {
            let mock_frontend = Arc::new(Mutex::new(MockFrontend::new()));
            let mut app = App::new(mock_frontend, temp_dir.clone())?;
            app.disable_lsp();
            callback(app, temp_dir)
        })
    }

    #[test]
    #[serial]
    fn copy_paste_from_different_file() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            let path_foo = temp_dir.join("src/foo.rs")?;

            // Open main.rs
            app.open_file(&path_main, true)?;

            // Copy the entire file
            app.handle_dispatch_editors(&[SelectWholeFile, Copy])?;

            // Open foo.rs
            app.open_file(&path_foo, true)?;

            // Copy the entire file
            app.handle_dispatch_editors(&[SelectWholeFile, Copy])?;

            // Open main.rs
            app.open_file(&path_main, true)?;

            // Select the entire file and paste
            app.handle_dispatch_editors(&[SelectWholeFile, Paste])?;

            // Expect the content of main.rs to be that of foo.rs
            let content_main = app.get_file_content(&path_main);
            let content_foo = app.get_file_content(&path_foo);
            assert_eq!(content_main, content_foo);
            Ok(())
        })
    }

    #[test]
    #[serial]
    fn copy_replace() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn main() { let x = 1; }".to_string()),
                SetSelectionMode(SelectionMode::BottomNode),
                Copy,
                MoveSelection(Movement::Next),
                Replace,
            ])?;

            assert_eq!(app.get_file_content(&path_main), "fn fn() { let x = 1; }");

            app.handle_dispatch_editors(&[Replace])?;

            assert_eq!(app.get_file_content(&path_main), "fn main() { let x = 1; }");
            assert_eq!(app.get_selected_texts(&path_main), vec!["main"]);

            Ok(())
        })
    }

    #[test]
    #[serial]
    fn copy_paste() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn main() { let x = 1; }".to_string()),
                SetSelectionMode(SelectionMode::BottomNode),
                Copy,
                MoveSelection(Movement::Next),
                Paste,
            ])?;

            assert_eq!(app.get_file_content(&path_main), "fn fn() { let x = 1; }");
            assert_eq!(app.get_selected_texts(&path_main), vec![""]);

            app.handle_dispatch_editors(&[MoveSelection(Movement::Next), Paste])?;

            assert_eq!(app.get_file_content(&path_main), "fn fn(fn { let x = 1; }");
            Ok(())
        })
    }

    #[test]
    #[serial]
    fn cut_paste() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn main() { let x = 1; }".to_string()),
                SetSelectionMode(SelectionMode::BottomNode),
                Cut,
            ])?;

            assert_eq!(app.get_file_content(&path_main), " main() { let x = 1; }");

            app.handle_dispatch_editors(&[MoveSelection(Movement::Current)])?;

            assert_eq!(app.get_selected_texts(&path_main), vec!["main"]);

            app.handle_dispatch_editors(&[Paste])?;

            assert_eq!(app.get_file_content(&path_main), " fn() { let x = 1; }");

            Ok(())
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_cut() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn f(){ let x = S(a); let y = S(b); }".to_string()),
                SetSelectionMode(SelectionMode::BottomNode),
                ToggleHighlightMode,
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
            ])?;

            assert_eq!(app.get_selected_texts(&path_main), vec!["fn f()"]);

            app.handle_dispatch_editors(&[Cut])?;

            assert_eq!(
                app.get_file_content(&path_main),
                "{ let x = S(a); let y = S(b); }"
            );

            app.handle_dispatch_editors(&[Paste])?;

            assert_eq!(
                app.get_file_content(&path_main),
                "fn f(){ let x = S(a); let y = S(b); }"
            );

            Ok(())
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_copy() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn f(){ let x = S(a); let y = S(b); }".to_string()),
                SetSelectionMode(SelectionMode::BottomNode),
                ToggleHighlightMode,
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
            ])?;
            assert_eq!(app.get_selected_texts(&path_main), vec!["fn f()"]);
            app.handle_dispatch_editors(&[Copy, MoveSelection(Movement::Next)])?;
            assert_eq!(app.get_selected_texts(&path_main), vec!["{"]);
            app.handle_dispatch_editors(&[Paste])?;
            assert_eq!(
                app.get_file_content(&path_main),
                "fn f()fn f() let x = S(a); let y = S(b); }"
            );
            Ok(())
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_replace() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn f(){ let x = S(a); let y = S(b); }".to_string()),
                SetSelectionMode(SelectionMode::BottomNode),
                ToggleHighlightMode,
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
            ])?;

            assert_eq!(app.get_selected_texts(&path_main), vec!["fn f()"]);

            app.handle_dispatch_editors(&[
                Copy,
                SetSelectionMode(SelectionMode::TopNode),
                MoveSelection(Movement::Next),
            ])?;

            assert_eq!(
                app.get_selected_texts(&path_main),
                vec!["{ let x = S(a); let y = S(b); }"]
            );

            app.handle_dispatch_editors(&[Replace])?;

            assert_eq!(app.get_file_content(&path_main), "fn f()fn f()");

            Ok(())
        })
    }

    #[test]
    #[serial]
    fn highlight_mode_paste() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            app.handle_dispatch_editors(&[
                SetContent("fn f(){ let x = S(a); let y = S(b); }".to_string()),
                SetSelectionMode(SelectionMode::BottomNode),
                ToggleHighlightMode,
                Copy,
            ])?;

            assert_eq!(app.get_selected_texts(&path_main), vec!["fn"]);

            app.handle_dispatch_editors(&[
                ToggleHighlightMode,
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
                MoveSelection(Movement::Next),
            ])?;

            assert_eq!(app.get_selected_texts(&path_main), vec!["fn f()"]);

            app.handle_dispatch_editors(&[Paste])?;

            assert_eq!(
                app.get_file_content(&path_main),
                "fn{ let x = S(a); let y = S(b); }"
            );

            Ok(())
        })
    }

    #[test]
    fn esc_should_close_signature_help() -> anyhow::Result<()> {
        run_test(|mut app, temp_dir| {
            let path_main = temp_dir.join("src/main.rs")?;
            app.open_file(&path_main, true)?;

            assert_eq!(app.components().len(), 1);

            app.handle_dispatch_editors(&[
                SetContent("fn f(){ let x = S(a); let y = S(b); }".to_string()),
                SetSelectionMode(SelectionMode::BottomNode),
                EnterInsertMode(Direction::End),
            ])?;

            let component_id = app.components()[0].borrow().id();
            app.handle_lsp_notification(LspNotification::SignatureHelp(
                crate::lsp::process::ResponseContext {
                    component_id,
                    request_kind: None,
                    description: None,
                },
                Some(crate::lsp::signature_help::SignatureHelp {
                    signatures: [SignatureInformation {
                        label: "Signature Help".to_string(),
                        documentation: Some(crate::lsp::documentation::Documentation {
                            content: "spongebob".to_string(),
                        }),
                        active_parameter_byte_range: None,
                    }]
                    .to_vec(),
                }),
            ))?;
            assert_eq!(app.components().len(), 2);

            app.handle_dispatch(Dispatch::HandleKeyEvent(key!("esc")))?;
            assert_eq!(app.components().len(), 1);

            Ok(())
        })
    }
}
