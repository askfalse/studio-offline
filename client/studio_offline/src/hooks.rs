use std::ffi::CStr;

pub type FromComponentsFn = extern "C" fn(
    res: *mut u128,
    schema: usize,
    host: usize,
    path: usize,
    query: usize,
    fragment: usize,
);
pub type TrustCheckFn = extern "C" fn(str1: *const i8, a2: i8, a3: i8) -> *mut u64;
pub type HttpRequestNotTrustedFn = extern "C" fn(a1: *mut usize, a2: usize) -> *mut i8;

pub static mut ORIGINAL: Option<FromComponentsFn> = None;
pub static mut OG_TC: Option<TrustCheckFn> = None;
pub static mut ORIGINAL_HTTP_NT: Option<HttpRequestNotTrustedFn> = None;

pub extern "C" fn hook_test(
    res: *mut u128,
    schema: usize,
    host: usize,
    path: usize,
    query: usize,
    fragment: usize,
) {
    unsafe {
        let str_host = "localhost:8081\0";
        let str_scheme = "http\0";

        *(host as *mut usize) = str_host.as_ptr() as usize;
        *(host as *mut usize).add(1) = str_host.len() - 1;

        *(schema as *mut usize) = str_scheme.as_ptr() as usize;
        *(schema as *mut usize).add(1) = str_scheme.len() - 1;

        if let Some(orig) = ORIGINAL {
            orig(res, schema, host, path, query, fragment);
        }
    }
}

pub extern "C" fn trustcheck_hook(str1: *const i8, a2: i8, a3: i8) -> *mut u64 {
    unsafe {
        let url = CStr::from_ptr(str1).to_string_lossy();
        let replacement = "http://roblox.com\0";

        if url.contains("http://localhost:8081") && a3 == 0 {
            if let Some(orig) = OG_TC {
                return orig(replacement.as_ptr() as *const i8, a2, a3);
            }
        }

        if let Some(orig) = OG_TC {
            return orig(str1, a2, a3);
        }
        std::ptr::null_mut()
    }
}

pub extern "C" fn nottrusted_hook(_a1: *mut usize, _a2: usize) -> *mut i8 {
    c"1".as_ptr() as *mut i8
}
