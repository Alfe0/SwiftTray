//! User-initiated release checks using the GitHub Releases API.
//!
//! No background polling, auto-download or auto-install occurs. The repository
//! is baked into official GitHub Actions builds by SWIFTTRAY_GITHUB_REPO.
//! Local builds without that setting report an unconfigured update source.
use serde::Deserialize;

const CURRENT: &str = env!("CARGO_PKG_VERSION");
const REPO: Option<&str> = option_env!("SWIFTTRAY_GITHUB_REPO");

#[derive(Debug)]
pub enum ResultState {
    Current { version: String },
    Available { version: String, url: String },
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
}

fn valid_repo(input: &str) -> bool {
    if input.len() > 200 { return false; }
    let mut parts = input.split('/');
    let valid_segment = |segment: &str| {
        !segment.is_empty() && segment.len() <= 100 && segment != "." && segment != ".."
            && segment.as_bytes().iter().all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'.' | b'_' | b'-'))
    };
    match (parts.next(), parts.next(), parts.next()) {
        (Some(owner), Some(name), None) => valid_segment(owner) && valid_segment(name),
        _ => false,
    }
}

pub fn repository_url() -> Option<String> {
    REPO.filter(|repo| valid_repo(repo)).map(|repo| format!("https://github.com/{repo}"))
}

pub fn is_configured() -> bool { REPO.is_some_and(valid_repo) }

fn release_url_valid(url: &str, repo: &str) -> bool {
    url.to_ascii_lowercase().starts_with(&format!("https://github.com/{repo}/releases/tag/").to_ascii_lowercase())
        && url.bytes().all(|b| b.is_ascii_graphic() && b != b'\\' && b != b'"')
}

// Compare ordinary stable release version components, not text (0.18.10 > 0.18.9).
fn version_tuple(value: &str) -> Option<[u32; 3]> {
    let core=value.trim_start_matches('v');
    let mut parts=core.split('.');
    let mut parsed=[0u32;3];
    for slot in &mut parsed { *slot=parts.next()?.parse().ok()?; }
    if parts.next().is_some() { return None; }
    Some(parsed)
}

pub fn check_latest() -> Result<ResultState, String> {
    let repo=REPO.filter(|repo| valid_repo(repo)).ok_or_else(||
        "Update source not configured. Official GitHub builds will use their repository.".to_string())?;
    let bytes=fetch_latest_json(repo)?;
    let release:Release=serde_json::from_slice(&bytes)
        .map_err(|_| "GitHub returned an unreadable release response".to_string())?;
    let remote=version_tuple(&release.tag_name)
        .ok_or_else(|| "Latest release has an unsupported version number".to_string())?;
    let current=version_tuple(CURRENT)
        .ok_or_else(|| "This build has an invalid version number".to_string())?;
    if remote > current {
        if !release_url_valid(&release.html_url,repo) {
            return Err("GitHub returned an unexpected release link".into());
        }
        Ok(ResultState::Available{version:release.tag_name,url:release.html_url})
    } else {
        Ok(ResultState::Current{version:CURRENT.to_string()})
    }
}

#[cfg(windows)]
fn fetch_latest_json(repo: &str) -> Result<Vec<u8>, String> {
    use std::{ffi::c_void,ptr};
    type Handle = *mut c_void;
    #[link(name="winhttp")]
    extern "system" {
        fn WinHttpOpen(agent:*const u16,access:u32,proxy:*const u16,bypass:*const u16,flags:u32)->Handle;
        fn WinHttpConnect(session:Handle,server:*const u16,port:u16,reserved:u32)->Handle;
        fn WinHttpOpenRequest(connection:Handle,verb:*const u16,path:*const u16,version:*const u16,
            referrer:*const u16,accept:*const *const u16,flags:u32)->Handle;
        fn WinHttpSetTimeouts(handle:Handle,resolve:i32,connect:i32,send:i32,receive:i32)->i32;
        fn WinHttpSendRequest(handle:Handle,headers:*const u16,header_length:u32,
            optional:*const c_void,optional_length:u32,total_length:u32,context:usize)->i32;
        fn WinHttpReceiveResponse(handle:Handle,reserved:*mut c_void)->i32;
        fn WinHttpQueryHeaders(handle:Handle,info:u32,name:*const u16,buffer:*mut c_void,
            length:*mut u32,index:*mut u32)->i32;
        fn WinHttpReadData(handle:Handle,buffer:*mut c_void,length:u32,read:*mut u32)->i32;
        fn WinHttpCloseHandle(handle:Handle)->i32;
    }
    struct Owned(Handle);
    impl Drop for Owned { fn drop(&mut self) { if !self.0.is_null() {unsafe {WinHttpCloseHandle(self.0);}} } }
    let wide=|s:&str|s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let agent=wide(&format!("SwiftTray/{CURRENT}"));
    let host=wide("api.github.com");
    let verb=wide("GET");
    let path=wide(&format!("/repos/{repo}/releases/latest"));
    let headers=wide("Accept: application/vnd.github+json\r\n");
    unsafe {
        let session=Owned(WinHttpOpen(agent.as_ptr(),0,ptr::null(),ptr::null(),0));
        if session.0.is_null() {return Err("Could not initialize Windows HTTPS".into());}
        WinHttpSetTimeouts(session.0,5000,5000,5000,8000);
        let connection=Owned(WinHttpConnect(session.0,host.as_ptr(),443,0));
        if connection.0.is_null() {return Err("Could not connect to GitHub".into());}
        let request=Owned(WinHttpOpenRequest(connection.0,verb.as_ptr(),path.as_ptr(),
            ptr::null(),ptr::null(),ptr::null(),0x0080_0000)); // WINHTTP_FLAG_SECURE
        if request.0.is_null() {return Err("Could not prepare the update check".into());}
        if WinHttpSendRequest(request.0,headers.as_ptr(),(headers.len()-1) as u32,
            ptr::null(),0,0,0)==0 || WinHttpReceiveResponse(request.0,ptr::null_mut())==0 {
            return Err("Could not contact GitHub. Check your internet connection.".into());
        }
        let mut status=0u32;let mut len=4u32;let mut index=0u32;
        if WinHttpQueryHeaders(request.0,19|0x2000_0000,ptr::null(),
            &mut status as *mut _ as *mut c_void,&mut len,&mut index)==0 {
            return Err("Could not read GitHub's response status".into());
        }
        if status==404 {return Err("No published GitHub Release was found yet".into());}
        if status!=200 {return Err(format!("GitHub release check failed (HTTP {status})"));}
        let mut result=Vec::new();
        loop {
            let mut buf=[0u8;4096];let mut read=0u32;
            if WinHttpReadData(request.0,buf.as_mut_ptr() as *mut c_void,
                buf.len() as u32,&mut read)==0 {
                return Err("Could not finish reading the release response".into());
            }
            if read==0 {break;}
            if result.len()+read as usize>1024*1024 {return Err("Release response too large".into());}
            result.extend_from_slice(&buf[..read as usize]);
        }
        Ok(result)
    }
}

#[cfg(not(windows))]
fn fetch_latest_json(_: &str) -> Result<Vec<u8>,String> {
    Err("Update checks require Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn version_order() {
        assert!(version_tuple("v0.18.10")>version_tuple("0.18.9"));
        assert_eq!(version_tuple("0.18.1"),Some([0,18,1]));
        assert_eq!(version_tuple("0.18.1-beta"),None);
    }
    #[test] fn repository_validation() {
        assert!(valid_repo("some-owner/SwiftTray"));
        assert!(!valid_repo("bad/path/extra"));
        assert!(!valid_repo("../repository"));
        assert!(!valid_repo("bad user/repo"));
    }
    #[test] fn release_link_validation() {
        assert!(release_url_valid("https://github.com/acme/swifttray/releases/tag/v0.18.1","acme/swifttray"));
        assert!(!release_url_valid("https://evil.example/tag/v1","acme/swifttray"));
    }
}
