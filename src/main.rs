use std::{
    env,
    error::Error,
    fs,
    io::{self, Read},
};

use native_dialog::MessageDialog;
use notify_rust::Notification;
use reqwest::{
    multipart::{Form, Part},
    Url, Version,
};
use serde_json::Value;

async fn real_main() -> color_eyre::Result<(), Box<dyn Error>> {
    let mut stdin = io::stdin().lock();
    let mut img_buf: Vec<u8> = vec![];
    stdin.read_to_end(&mut img_buf)?;

    // TODO: Check if valid!

    let should_upload = MessageDialog::new()
        .set_type(native_dialog::MessageType::Info)
        .set_title("Upload?")
        .set_text("Do you want to upload this image?")
        .show_confirm()
        .unwrap();

    if !should_upload {
        let mut opts = wl_clipboard_rs::copy::Options::new();
        opts.clipboard(wl_clipboard_rs::copy::ClipboardType::Both);
        opts.foreground(true); // hang program to make sure you always can copy

        Notification::new()
            .summary("Screenshot")
            .body("Screenshot has been copied to clipboard.")
            .appname("filebin-img-uploader")
            .timeout(5000)
            .show()?;

        opts.copy(
            wl_clipboard_rs::copy::Source::Bytes(img_buf.into_boxed_slice()),
            wl_clipboard_rs::copy::MimeType::Autodetect,
        )?;

        return Ok(());
    }

    let mime = infer::get(&img_buf).ok_or("Couldn't recognize file type")?;

    let mut filebin_upload_endpoint = Url::parse(&env::var("FILEBIN_ENDPOINT")?)?;
    filebin_upload_endpoint.set_path("/api/file");

    Notification::new()
        .summary("Screenshot")
        .body("Uploading to filebin...")
        .appname("filebin-img-uploader")
        .timeout(5000)
        .show()?;

    let mime_type = mime.mime_type();

    let form = Form::new().part(
        "file",
        Part::bytes(img_buf)
            .file_name(format!("upload.{}", mime.extension()))
            .mime_str(mime_type)?, // .mime_str(mime.mime_type())?,
    );

    let client = reqwest::Client::new();
    let res = client
        .post(filebin_upload_endpoint)
        .multipart(form)
        .version(Version::HTTP_11)
        .send()
        .await?
        .error_for_status()?;

    let res_json: Value = serde_json::from_str(&res.text().await?)?;

    let mut filebin_download_endpoint = Url::parse(&env::var("FILEBIN_ENDPOINT")?)?;
    filebin_download_endpoint.set_path(&format!(
        "/api/file/{}",
        res_json["id"]
            .as_str()
            .ok_or("Couldn't find id in server response")?
    ));

    let mut opts = wl_clipboard_rs::copy::Options::new();
    opts.clipboard(wl_clipboard_rs::copy::ClipboardType::Both);
    opts.foreground(true); // hang program to make sure you always can copy

    Notification::new()
        .summary("Screenshot")
        .body(&format!(
            "Screenshot link has been copied to clipboard. ({})",
            filebin_download_endpoint.to_string()
        ))
        .appname("filebin-img-uploader")
        .timeout(5000)
        .show()?;

    opts.copy(
        wl_clipboard_rs::copy::Source::Bytes(
            filebin_download_endpoint.to_string().into_bytes().into(),
        ),
        wl_clipboard_rs::copy::MimeType::Autodetect,
    )?;

    Ok(())
}

#[tokio::main]
async fn main() -> color_eyre::Result<(), Box<dyn Error>> {
    color_eyre::install()?;

    match real_main().await {
        Ok(_) => return Ok(()),
        Err(e) => {
            Notification::new()
                .summary("Screenshot")
                .body(&format!("Image uploader failed with {:?}", e.to_string()))
                .appname("filebin-img-uploader")
                .timeout(5000)
                .show()?;
            Err::<(), Box<dyn std::error::Error>>(e)?;
        }
    }

    Ok(())
}
