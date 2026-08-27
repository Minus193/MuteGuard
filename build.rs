use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/muteguard.ico");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("gnu")
    {
        return;
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let icon_path = manifest_dir.join("assets").join("muteguard.ico");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let resource_script = out_dir.join("muteguard-icon.rc");
    let icon_path = icon_path
        .canonicalize()
        .expect("locate assets/muteguard.ico")
        .to_string_lossy()
        .replace('\\', "/");
    let version = env::var("CARGO_PKG_VERSION").expect("read package version");
    let mut version_parts = version
        .split('.')
        .map(|part| part.parse::<u16>().unwrap_or(0))
        .chain(std::iter::repeat(0))
        .take(4);
    let version_tuple = format!(
        "{},{},{},{}",
        version_parts.next().unwrap(),
        version_parts.next().unwrap(),
        version_parts.next().unwrap(),
        version_parts.next().unwrap()
    );
    fs::write(
        &resource_script,
        format!(
            r#"1 ICON "{icon_path}"
1 VERSIONINFO
FILEVERSION {version_tuple}
PRODUCTVERSION {version_tuple}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "CompanyName", "MuteGuard\0"
            VALUE "FileDescription", "MuteGuard\0"
            VALUE "FileVersion", "{version}\0"
            VALUE "InternalName", "muteguard\0"
            VALUE "LegalCopyright", "Licensed under Apache-2.0\0"
            VALUE "OriginalFilename", "muteguard.exe\0"
            VALUE "ProductName", "MuteGuard\0"
            VALUE "ProductVersion", "{version}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
        ),
    )
    .expect("write Windows resource script");

    let resource_object = out_dir.join("muteguard-resource.o");
    run_resource_compiler(
        Command::new("x86_64-w64-mingw32-windres")
            .arg("--input")
            .arg(&resource_script)
            .arg("--output")
            .arg(&resource_object)
            .arg("--output-format=coff"),
    );

    let resource_library = out_dir.join("libmuteguard_resource.a");
    run_resource_compiler(
        Command::new("x86_64-w64-mingw32-ar")
            .arg("rcs")
            .arg(&resource_library)
            .arg(&resource_object),
    );

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    // A COFF resource object does not expose a Rust-referenced symbol, so GNU
    // ld would otherwise discard this archive as unused.
    println!("cargo:rustc-link-lib=static:+whole-archive=muteguard_resource");
}

fn run_resource_compiler(command: &mut Command) {
    let status = command
        .status()
        .expect("start the Windows resource compiler");
    assert!(status.success(), "Windows resource compiler failed");
}
