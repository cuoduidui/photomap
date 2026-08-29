//! 敏感配置的本地加密
//!
//! 值以 `enc:v1:<base64>` 的形式存入数据库，读取时自动解密。
//! - Windows：DPAPI 加密，密钥绑定当前 Windows 用户
//! - macOS：主密钥保存在系统钥匙串（Keychain），使用 ChaCha20-Poly1305 加密
//! - 其他平台：降级为明文存储并输出警告

const ENC_PREFIX: &str = "enc:v1:";

pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENC_PREFIX)
}

pub fn encrypt(plain: &str) -> Result<String, String> {
    backend::encrypt(plain)
}

pub fn decrypt(stored: &str) -> Result<String, String> {
    if !is_encrypted(stored) {
        return Err("不是加密配置值".to_string());
    }
    backend::decrypt(stored)
}

#[cfg(target_os = "windows")]
mod backend {
    use super::ENC_PREFIX;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

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
}

#[cfg(target_os = "macos")]
mod backend {
    use super::ENC_PREFIX;
    use base64::Engine;
    use chacha20poly1305::aead::{Aead, KeyInit, Payload};
    use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

    const SERVICE: &str = "com.cuoduidui.photomap";
    const ACCOUNT: &str = "photomap-master-key";

    fn master_key() -> Result<[u8; 32], String> {
        let entry = keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(stored) => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(stored.trim())
                    .map_err(|e| format!("主密钥格式错误: {}", e))?;
                if bytes.len() != 32 {
                    return Err("主密钥长度异常".to_string());
                }
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                Ok(key)
            }
            Err(_) => {
                let mut key = [0u8; 32];
                getrandom::fill(&mut key).map_err(|e| e.to_string())?;
                let stored = base64::engine::general_purpose::STANDARD.encode(key);
                entry.set_password(&stored).map_err(|e| e.to_string())?;
                Ok(key)
            }
        }
    }

    pub fn encrypt(plain: &str) -> Result<String, String> {
        let key = match master_key() {
            Ok(k) => k,
            Err(e) => {
                eprintln!("钥匙串不可用，敏感配置将按明文存储: {}", e);
                return Ok(plain.to_string());
            }
        };
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        let mut nonce = [0u8; 12];
        getrandom::fill(&mut nonce).map_err(|e| e.to_string())?;
        let ct = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: plain.as_bytes(),
                    aad: b"photomap",
                },
            )
            .map_err(|e| format!("加密失败: {}", e))?;
        let mut out = nonce.to_vec();
        out.extend_from_slice(&ct);
        Ok(format!(
            "{}{}",
            ENC_PREFIX,
            base64::engine::general_purpose::STANDARD.encode(out)
        ))
    }

    pub fn decrypt(stored: &str) -> Result<String, String> {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(&stored[ENC_PREFIX.len()..])
            .map_err(|e| format!("加密数据格式错误: {}", e))?;
        if raw.len() < 12 {
            return Err("加密数据长度异常".to_string());
        }
        let (nonce, ct) = raw.split_at(12);
        let key = master_key()?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        let pt = cipher
            .decrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: ct,
                    aad: b"photomap",
                },
            )
            .map_err(|e| format!("解密失败: {}", e))?;
        String::from_utf8(pt).map_err(|e| format!("解密结果不是有效文本: {}", e))
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod backend {
    pub fn encrypt(plain: &str) -> Result<String, String> {
        eprintln!("当前平台不支持安全存储，敏感配置将按明文保存");
        Ok(plain.to_string())
    }

    pub fn decrypt(stored: &str) -> Result<String, String> {
        Ok(stored.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let plain = "sk-test-中文密钥-123";
        let stored = encrypt(plain).unwrap();
        assert!(is_encrypted(&stored));
        assert!(!stored.contains(plain));
        assert_eq!(decrypt(&stored).unwrap(), plain);
    }
}
