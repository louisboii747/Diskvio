use tauri::{
    App, Emitter,
    menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
};

pub fn install(app: &App) -> tauri::Result<()> {
    let about = AboutMetadata {
        name: Some("Diskvio".into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        comments: Some("Milestone 1 — demonstration devices only. No disk operations.".into()),
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

    let refresh = MenuItemBuilder::with_id("refresh", "Refresh Demonstration Data")
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
    let overview = MenuItemBuilder::with_id("overview", "Disk Overview")
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
            "Erase Disk — Not Yet Available",
        )?)
        .item(&unavailable("eject-disk", "Eject — Not Yet Available")?)
        .build()?;
    let partition = SubmenuBuilder::new(app, "Partition")
        .item(&unavailable(
            "create-partition",
            "Create — Not Yet Available",
        )?)
        .item(&unavailable(
            "format-partition",
            "Format — Not Yet Available",
        )?)
        .item(&unavailable(
            "delete-partition",
            "Delete — Not Yet Available",
        )?)
        .item(&unavailable(
            "resize-partition",
            "Resize — Not Yet Available",
        )?)
        .build()?;
    let usb_menu = SubmenuBuilder::new(app, "USB")
        .item(&unavailable(
            "write-image",
            "Write Image — Not Yet Available",
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
