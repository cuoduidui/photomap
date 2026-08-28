//! 敏感配置的本地加密（Windows DPAPI）
//!
//! 值以 `enc:v1:<base64>` 的形式存入数据库，读取时自动解密。
//! DPAPI 将密钥绑定到当前 Windows 用户，同一账号下可透明加解密；
//! 其他账号/机器即使拿到数据库文件也无法还原明文。

use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
};

const ENC_PREFIX: &str = "enc:v1:";

pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENC_PREFIX)
}

/// 用 DPAPI 加密一段文本（按 UTF-16LE 存储，保证中文等字符可还原）
pub fn encrypt(plain: &str) -> Result<String, String> {
    let plain_utf16: Vec<u16> = plain.encode_utf16().collect();
    let in_blob = CRYPT_INTEGER_BLOB {
        cbData: (plain_utf16.len() * 2) as u32,
        pbData: plain_utf16.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB::default();

    let ok = unsafe {
        CryptProtectData(
            &in_blob,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
    };
    if ok == 0 {
        return Err("DPAPI 加密失败".to_string());
    }

    let cipher =
        unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(out_blob.pbData as _);
    }

    use base64::Engine;
    Ok(format!(
        "{}{}",
        ENC_PREFIX,
        base64::engine::general_purpose::STANDARD.encode(cipher)
    ))
}

/// 解密 `enc:v1:` 前缀的配置值
pub fn decrypt(stored: &str) -> Result<String, String> {
    use base64::Engine;

    let cipher = base64::engine::general_purpose::STANDARD
        .decode(&stored[ENC_PREFIX.len()..])
        .map_err(|e| format!("加密数据格式错误: {}", e))?;

    let in_blob = CRYPT_INTEGER_BLOB {
        cbData: cipher.len() as u32,
        pbData: cipher.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB::default();

    let ok = unsafe {
        CryptUnprotectData(
            &in_blob,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
    };
    if ok == 0 {
        return Err("DPAPI 解密失败".to_string());
    }

    let bytes =
        unsafe { std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(out_blob.pbData as _);
    }

    if bytes.len() % 2 != 0 {
        return Err("解密数据长度异常".to_string());
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16(&units).map_err(|e| format!("解密结果不是有效文本: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpapi_round_trip() {
        let plain = "sk-test-中文密钥-123";
        let stored = encrypt(plain).unwrap();
        assert!(is_encrypted(&stored));
        assert!(!stored.contains(plain));
        assert_eq!(decrypt(&stored).unwrap(), plain);
    }
}
