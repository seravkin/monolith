use base64;
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use url::{form_urlencoded, ParseError, Url};

const MAGIC: [[&[u8]; 2]; 18] = [
    // Image
    [b"GIF87a", b"image/gif"],
    [b"GIF89a", b"image/gif"],
    [b"\xFF\xD8\xFF", b"image/jpeg"],
    [b"\x89PNG\x0D\x0A\x1A\x0A", b"image/png"],
    [b"<svg ", b"image/svg+xml"],
    [b"RIFF....WEBPVP8 ", b"image/webp"],
    [b"\x00\x00\x01\x00", b"image/x-icon"],
    // Audio
    [b"ID3", b"audio/mpeg"],
    [b"\xFF\x0E", b"audio/mpeg"],
    [b"\xFF\x0F", b"audio/mpeg"],
    [b"OggS", b"audio/ogg"],
    [b"RIFF....WAVEfmt ", b"audio/wav"],
    [b"fLaC", b"audio/x-flac"],
    // Video
    [b"RIFF....AVI LIST", b"video/avi"],
    [b"....ftyp", b"video/mp4"],
    [b"\x00\x00\x01\x0B", b"video/mpeg"],
    [b"....moov", b"video/quicktime"],
    [b"\x1A\x45\xDF\xA3", b"video/webm"],
];

const PLAINTEXT_MEDIA_TYPES: &[&str] = &[
    "image/svg+xml",
    "text/css",
    "text/html",
    "text/javascript",
    "text/plain",
];

pub fn data_to_data_url(media_type: &str, data: &[u8], url: &str, fragment: &str) -> String {
    let media_type: String = if media_type.is_empty() {
        detect_media_type(data, &url)
    } else {
        media_type.to_string()
    };
    let hash: String = if fragment != "" {
        format!("#{}", fragment)
    } else {
        str!()
    };

    format!(
        "data:{};base64,{}{}",
        media_type,
        base64::encode(data),
        hash
    )
}

pub fn detect_media_type(data: &[u8], url: &str) -> String {
    for item in MAGIC.iter() {
        if data.starts_with(item[0]) {
            return String::from_utf8(item[1].to_vec()).unwrap();
        }
    }

    if url.to_lowercase().ends_with(".svg") {
        return str!("image/svg+xml");
    }

    str!()
}

pub fn url_has_protocol<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme().len() > 0))
        .unwrap_or(false)
}

pub fn is_data_url<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme() == "data"))
        .unwrap_or(false)
}

pub fn is_file_url<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme() == "file"))
        .unwrap_or(false)
}

pub fn is_http_url<T: AsRef<str>>(url: T) -> bool {
    Url::parse(url.as_ref())
        .and_then(|u| Ok(u.scheme() == "http" || u.scheme() == "https"))
        .unwrap_or(false)
}

pub fn is_plaintext_media_type(media_type: &str) -> bool {
    PLAINTEXT_MEDIA_TYPES.contains(&media_type.to_lowercase().as_str())
}

pub fn resolve_url<T: AsRef<str>, U: AsRef<str>>(from: T, to: U) -> Result<String, ParseError> {
    let result = if is_http_url(to.as_ref()) {
        to.as_ref().to_string()
    } else {
        Url::parse(from.as_ref())?
            .join(to.as_ref())?
            .as_ref()
            .to_string()
    };
    Ok(result)
}

pub fn get_url_fragment<T: AsRef<str>>(url: T) -> String {
    if Url::parse(url.as_ref()).unwrap().fragment() == None {
        str!()
    } else {
        str!(Url::parse(url.as_ref()).unwrap().fragment().unwrap())
    }
}

pub fn clean_url<T: AsRef<str>>(input: T) -> String {
    let mut url = Url::parse(input.as_ref()).unwrap();

    // Clear fragment
    url.set_fragment(None);

    // Get rid of stray question mark
    if url.query() == Some("") {
        url.set_query(None);
    }

    // Remove empty trailing ampersand(s)
    let mut result: String = url.to_string();
    while result.ends_with("&") {
        result.pop();
    }

    result
}

pub fn data_url_to_data<T: AsRef<str>>(url: T) -> (String, Vec<u8>) {
    let parsed_url: Url = Url::parse(url.as_ref()).unwrap_or(Url::parse("data:,").unwrap());
    let path: String = parsed_url.path().to_string();
    let comma_loc: usize = path.find(',').unwrap_or(path.len());

    let meta_data: String = path.chars().take(comma_loc).collect();
    let raw_data: String = path.chars().skip(comma_loc + 1).collect();

    let text: String = decode_url(raw_data);

    let meta_data_items: Vec<&str> = meta_data.split(';').collect();
    let mut media_type: String = str!();
    let mut encoding: &str = "";

    let mut i: i8 = 0;
    for item in &meta_data_items {
        if i == 0 {
            media_type = str!(item);
        } else {
            if item.eq_ignore_ascii_case("base64")
                || item.eq_ignore_ascii_case("utf8")
                || item.eq_ignore_ascii_case("charset=UTF-8")
            {
                encoding = item;
            }
        }

        i = i + 1;
    }

    let data: Vec<u8> = if encoding.eq_ignore_ascii_case("base64") {
        base64::decode(&text).unwrap_or(vec![])
    } else {
        text.as_bytes().to_vec()
    };

    (media_type, data)
}

pub fn decode_url(input: String) -> String {
    let input: String = input.replace("+", "%2B");

    form_urlencoded::parse(input.as_bytes())
        .map(|(key, val)| {
            [
                key.to_string(),
                if val.to_string().len() == 0 {
                    str!()
                } else {
                    str!('=')
                },
                val.to_string(),
            ]
            .concat()
        })
        .collect()
}

pub fn file_url_to_fs_path(url: &str) -> String {
    if !is_file_url(url) {
        return str!();
    }

    let cutoff_l = if cfg!(windows) { 8 } else { 7 };
    let mut fs_file_path: String = decode_url(url.to_string()[cutoff_l..].to_string());
    let url_fragment = get_url_fragment(url);
    if url_fragment != "" {
        let max_len = fs_file_path.len() - 1 - url_fragment.len();
        fs_file_path = fs_file_path[0..max_len].to_string();
    }

    if cfg!(windows) {
        fs_file_path = fs_file_path.replace("/", "\\");
    }

    // File paths should not be %-encoded
    decode_url(fs_file_path)
}

pub fn retrieve_asset(
    cache: &mut HashMap<String, Vec<u8>>,
    client: &Client,
    parent_url: &str,
    url: &str,
    opt_silent: bool,
) -> Result<(Vec<u8>, String, String), reqwest::Error> {
    if url.len() == 0 {
        // Provoke error
        client.get("").send()?;
    }

    if is_data_url(&url) {
        let (media_type, data) = data_url_to_data(url);
        Ok((data, url.to_string(), media_type))
    } else if is_file_url(&url) {
        // Check if parent_url is also file:///
        // (if not, then we don't embed the asset)
        if !is_file_url(&parent_url) {
            // Provoke error
            client.get("").send()?;
        }

        let fs_file_path: String = file_url_to_fs_path(url);
        let path = Path::new(&fs_file_path);
        if path.exists() {
            if !opt_silent {
                eprintln!("{}", &url);
            }

            Ok((fs::read(&fs_file_path).expect(""), url.to_string(), str!()))
        } else {
            // Provoke error
            Err(client.get("").send().unwrap_err())
        }
    } else {
        let cache_key: String = clean_url(&url);

        if cache.contains_key(&cache_key) {
            // URL is in cache, we get and return it
            if !opt_silent {
                eprintln!("{} (from cache)", &url);
            }

            Ok((
                cache.get(&cache_key).unwrap().to_vec(),
                url.to_string(),
                str!(),
            ))
        } else {
            // URL not in cache, we retrieve the file
            let mut response = client.get(url).send()?;
            let res_url = response.url().to_string();

            if !opt_silent {
                if url == res_url {
                    eprintln!("{}", &url);
                } else {
                    eprintln!("{} -> {}", &url, &res_url);
                }
            }

            let new_cache_key: String = clean_url(&res_url);

            // Convert response into a byte array
            let mut data: Vec<u8> = vec![];
            response.copy_to(&mut data)?;

            // Attempt to obtain media type by reading the Content-Type header
            let media_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|header| header.to_str().ok())
                .unwrap_or("");

            // Add to cache
            cache.insert(new_cache_key, data.clone());

            Ok((data, res_url, media_type.to_string()))
        }
    }
}
