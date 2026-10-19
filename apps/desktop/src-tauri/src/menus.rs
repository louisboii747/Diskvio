use tauri::{
    App, Emitter,
    menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
};

pub fn install(app: &App) -> tauri::Result<()> {
    let about = AboutMetadata {
        name: Some("Diskvio".into()),
        // Empty values prevent macOS from filling these in from the bundle metadata.
        #[cfg(target_os = "macos")]
        version: Some(String::new()),
        #[cfg(target_os = "macos")]
        short_version: Some(String::new()),
        comments: Some("Explore sample drives. Your files stay untouched.".into()),
        license: Some("MIT".into()),
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    let application = SubmenuBuilder::new(app, "Diskvio")
        .about(Some(about.clone()))
        .separator()
        .item(
            &MenuItemBuilder::with_id("settings", "Settings…")
                .accelerator("CmdOrCtrl+,")
                .build(app)?,
        )
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let refresh = MenuItemBuilder::with_id("refresh", "Reload Sample Drives")
        .accelerator("CmdOrCtrl+R")
        .build(app)?;
    let file_builder = SubmenuBuilder::new(app, "File").item(&refresh);
    #[cfg(not(target_os = "macos"))]
    let file_builder = file_builder.separator().quit();
    let file = file_builder.build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let overview = MenuItemBuilder::with_id("overview", "Drives")
        .accelerator("CmdOrCtrl+1")
        .build(app)?;
    let partitions = MenuItemBuilder::with_id("partitions", "Partitions")
        .accelerator("CmdOrCtrl+2")
        .build(app)?;
    let usb = MenuItemBuilder::with_id("usb", "USB Writer")
        .accelerator("CmdOrCtrl+3")
        .build(app)?;
    let view_builder = SubmenuBuilder::new(app, "View").items(&[&overview, &partitions, &usb]);
    #[cfg(not(target_os = "macos"))]
    let view_builder = view_builder.separator().item(
        &MenuItemBuilder::with_id("settings", "Settings…")
            .accelerator("CmdOrCtrl+,")
            .build(app)?,
    );
    let view = view_builder.build()?;

    let unavailable = |id, label| {
        MenuItemBuilder::with_id(id, label)
            .enabled(false)
            .build(app)
    };
    let disk = SubmenuBuilder::new(app, "Disk")
        .item(&unavailable(
            "erase-disk",
            "Erase Drive (not available yet)",
        )?)
        .item(&unavailable("eject-disk", "Eject (not available yet)")?)
        .build()?;
    let partition = SubmenuBuilder::new(app, "Partition")
        .item(&unavailable(
            "create-partition",
            "Create (not available yet)",
        )?)
        .item(&unavailable(
            "format-partition",
            "Format (not available yet)",
        )?)
        .item(&unavailable(
            "delete-partition",
            "Delete (not available yet)",
        )?)
        .item(&unavailable(
            "resize-partition",
            "Resize (not available yet)",
        )?)
        .build()?;
    let usb_menu = SubmenuBuilder::new(app, "USB")
        .item(&unavailable(
            "write-image",
            "Write to USB (not available yet)",
        )?)
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .close_window()
        .build()?;
    let help = SubmenuBuilder::new(app, "Help")
        .about(Some(about))
        .build()?;
    let builder = MenuBuilder::new(app);
    #[cfg(target_os = "macos")]
    let builder = builder.item(&application);
    let menu = builder
        .items(&[
            &file, &edit, &view, &disk, &partition, &usb_menu, &window, &help,
        ])
        .build()?;
    app.set_menu(menu)?;
    #[cfg(target_os = "macos")]
    {
        window.set_as_windows_menu_for_nsapp()?;
        help.set_as_help_menu_for_nsapp()?;
    }
    app.on_menu_event(|app, event| {
        let id = event.id().as_ref();
        if matches!(
            id,
            "refresh" | "overview" | "partitions" | "usb" | "settings"
        ) && let Err(error) = app.emit_to("main", "diskvio://menu", id)
        {
            eprintln!("Could not deliver native menu action: {error}");
        }
    });
    Ok(())
}
