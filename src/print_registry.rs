use windows::core::GUID;
use windows::Win32::UI::Shell::SHGetKnownFolderPath;
use windows::Win32::UI::Shell::{
    FOLDERID_Profile,
    FOLDERID_Favorites,
    FOLDERID_Desktop,
    FOLDERID_Documents,
    FOLDERID_Music,
    FOLDERID_Pictures,
    FOLDERID_SavedGames,
    FOLDERID_Videos,
    FOLDERID_RoamingAppData,
    FOLDERID_RecycleBinFolder,
    FOLDERID_CommonStartup,
    FOLDERID_ProgramData,
    FOLDERID_PublicDesktop,
    FOLDERID_PublicDocuments,
    FOLDERID_ProgramFiles,
    FOLDERID_ProgramFilesX86,
    FOLDERID_ProgramFilesCommon,
    FOLDERID_ProgramFilesCommonX86,
    FOLDERID_Programs,
    FOLDERID_Windows,
};
use windows::Win32::UI::Shell::KF_FLAG_CREATE;
use windows::Win32::System::Com::CoTaskMemFree;

unsafe fn string_from_wchar_pointer(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut length = 0;
    while ptr.add(length).read() != 0 {
        length += 1;
    }
    let slice = std::slice::from_raw_parts(ptr, length);
    String::from_utf16_lossy(slice)
}

fn get_registry_folder_string(id: GUID) -> anyhow::Result<String> {
    unsafe {
        let known_folder_flags = KF_FLAG_CREATE;
        let result = SHGetKnownFolderPath(&id, known_folder_flags, None);
        let path: anyhow::Result<String> = result.clone()
            .map(|address| string_from_wchar_pointer(address.0))
            .map_err(|err| err.clone().into());
        let address = result.ok().map(|ptr| ptr.0 as *const std::ffi::c_void);
        CoTaskMemFree(address);
        path
    }
}

#[cfg(not(target_os = "windows"))]
compile_error!("Windows-only binary");

#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .with_colors(true)
        .without_timestamps()
        .init()?;

    let folders = vec![
        ("Profile", FOLDERID_Profile),
        ("Favorites", FOLDERID_Favorites),
        ("Desktop", FOLDERID_Desktop),
        ("Documents", FOLDERID_Documents),
        ("Music", FOLDERID_Music),
        ("Pictures", FOLDERID_Pictures),
        ("SavedGames", FOLDERID_SavedGames),
        ("Videos", FOLDERID_Videos),
        ("RoamingAppData", FOLDERID_RoamingAppData),
        ("RecycleBinFolder", FOLDERID_RecycleBinFolder),
        ("CommonStartup", FOLDERID_CommonStartup),
        ("ProgramData", FOLDERID_ProgramData),
        ("PublicDesktop", FOLDERID_PublicDesktop),
        ("PublicDocuments", FOLDERID_PublicDocuments),
        ("ProgramFiles", FOLDERID_ProgramFiles),
        ("ProgramFilesX86", FOLDERID_ProgramFilesX86),
        ("ProgramFilesCommon", FOLDERID_ProgramFilesCommon),
        ("ProgramFilesCommonX86", FOLDERID_ProgramFilesCommonX86),
        ("Programs", FOLDERID_Programs),
        ("Windows", FOLDERID_Windows),
    ];
    for (name, id) in folders {
        match get_registry_folder_string(id) {
            Ok(string) => println!("{name} = {string}"),
            Err(err) => log::error!("Failed to read registry entry for {name}: {err}"),
        }
    }

    Ok(())
}
