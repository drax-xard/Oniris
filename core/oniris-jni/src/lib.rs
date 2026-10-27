//! JNI bridge for the Android shell.
//!
//! The Kotlin side serializes all calls: the engine pointer is not internally
//! synchronized. The render call writes directly into a direct ByteBuffer owned
//! by the audio thread, so there is no per-sample JNI overhead.

use std::path::Path;
use std::time::Duration;

use jni::objects::{JByteBuffer, JObject, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;
use oniris_core::{Engine, MixSpec};
use oniris_pack::install_archive;

#[no_mangle]
pub extern "system" fn Java_dev_oniris_data_NativePack_nativeInstallArchive(
    mut env: JNIEnv,
    _this: JObject,
    archive_path: JString,
    packs_dir: JString,
) -> jstring {
    let archive: String = match env.get_string(&archive_path) {
        Ok(value) => value.into(),
        Err(error) => return fail_string(&mut env, format!("invalid archive path: {error}")),
    };
    let packs_dir: String = match env.get_string(&packs_dir) {
        Ok(value) => value.into(),
        Err(error) => return fail_string(&mut env, format!("invalid packs directory: {error}")),
    };

    let installed = match install_archive(Path::new(&archive), Path::new(&packs_dir)) {
        Ok(value) => value,
        Err(error) => return fail_string(&mut env, error.to_string()),
    };

    // The archive checks the manifest and file presence; decoding with a real
    // engine also validates loop bounds, codecs, and the memory budget. Remove
    // the published directory if the audio is unusable.
    let mut engine = Engine::new(48_000);
    if let Err(error) = engine.load_pack(&installed.directory) {
        let _ = std::fs::remove_dir_all(&installed.directory);
        return fail_string(&mut env, format!("invalid pack audio: {error}"));
    }

    match env.new_string(&installed.info.manifest.id) {
        Ok(id) => id.into_raw(),
        Err(error) => fail_string(&mut env, format!("could not return the pack id: {error}")),
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeCreate(
    _env: JNIEnv,
    _this: JObject,
    sample_rate: jint,
) -> jlong {
    Box::into_raw(Box::new(Engine::new(sample_rate as u32))) as jlong
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeDestroy(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) {
    if handle != 0 {
        unsafe {
            drop(Box::from_raw(handle as *mut Engine));
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeLoadPack(
    mut env: JNIEnv,
    _this: JObject,
    handle: jlong,
    path: JString,
) -> jint {
    let Some(engine) = engine_mut(handle) else {
        return fail(&mut env, "invalid engine handle", -1);
    };
    let path: String = match env.get_string(&path) {
        Ok(value) => value.into(),
        Err(error) => return fail(&mut env, format!("invalid path: {error}"), -2),
    };

    match engine.load_pack(Path::new(&path)) {
        Ok(_) => 0,
        Err(error) => fail(&mut env, error.to_string(), -3),
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeSetMix(
    mut env: JNIEnv,
    _this: JObject,
    handle: jlong,
    json: JString,
) -> jint {
    let Some(engine) = engine_mut(handle) else {
        return fail(&mut env, "invalid engine handle", -1);
    };
    let json: String = match env.get_string(&json) {
        Ok(value) => value.into(),
        Err(error) => return fail(&mut env, format!("invalid mix JSON: {error}"), -2),
    };
    let mix: MixSpec = match serde_json::from_str(&json) {
        Ok(value) => value,
        Err(error) => return fail(&mut env, format!("invalid mix JSON: {error}"), -3),
    };

    match engine.set_mix(&mix) {
        Ok(()) => 0,
        Err(error) => fail(&mut env, error.to_string(), -4),
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeSetPackGain(
    mut env: JNIEnv,
    _this: JObject,
    handle: jlong,
    id: JString,
    gain: f32,
) -> jint {
    let Some(engine) = engine_mut(handle) else {
        return fail(&mut env, "invalid engine handle", -1);
    };
    let id: String = match env.get_string(&id) {
        Ok(value) => value.into(),
        Err(error) => return fail(&mut env, format!("invalid pack id: {error}"), -2),
    };

    match engine.set_pack_gain(&id, gain) {
        Ok(()) => 0,
        Err(error) => fail(&mut env, error.to_string(), -3),
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeSetMasterGain(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
    gain: f32,
) {
    if let Some(engine) = engine_mut(handle) {
        engine.set_master_gain(gain);
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativePlay(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) {
    if let Some(engine) = engine_mut(handle) {
        engine.play();
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativePause(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) {
    if let Some(engine) = engine_mut(handle) {
        engine.pause();
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeStop(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) {
    if let Some(engine) = engine_mut(handle) {
        engine.stop();
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeSetSleepTimer(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
    duration_ms: jlong,
    fade_ms: jlong,
) {
    if let Some(engine) = engine_mut(handle) {
        engine.set_sleep_timer(
            Duration::from_millis(duration_ms.max(0) as u64),
            Duration::from_millis(fade_ms.max(0) as u64),
        );
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeSetFadeIn(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
    duration_ms: jlong,
) {
    if let Some(engine) = engine_mut(handle) {
        engine.set_fade_in(Duration::from_millis(duration_ms.max(0) as u64));
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeClearSleepTimer(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) {
    if let Some(engine) = engine_mut(handle) {
        engine.clear_sleep_timer();
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeRender(
    mut env: JNIEnv,
    _this: JObject,
    handle: jlong,
    buffer: JByteBuffer,
    frames: jint,
) -> jint {
    let Some(engine) = engine_mut(handle) else {
        return fail(&mut env, "invalid engine handle", -1);
    };
    if frames <= 0 {
        return 0;
    }
    let frames = frames as usize;

    let capacity = match env.get_direct_buffer_capacity(&buffer) {
        Ok(value) => value,
        Err(error) => return fail(&mut env, format!("invalid direct buffer: {error}"), -2),
    };
    let required = frames * 4; // stereo i16
    if capacity < required {
        return fail(
            &mut env,
            format!("direct buffer too small: {capacity} < {required} bytes"),
            -3,
        );
    }

    let address = match env.get_direct_buffer_address(&buffer) {
        Ok(value) => value,
        Err(error) => return fail(&mut env, format!("invalid direct buffer: {error}"), -4),
    };
    if address.is_null() {
        return fail(&mut env, "direct buffer is null", -5);
    }

    let output = unsafe { std::slice::from_raw_parts_mut(address as *mut i16, frames * 2) };
    let stats = engine.render_i16(output, frames);
    stats.frames_written as jint
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeIsFinished(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) -> jboolean {
    engine_mut(handle)
        .map(|engine| engine.is_finished() as jboolean)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeIsPaused(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) -> jboolean {
    engine_mut(handle)
        .map(|engine| engine.is_paused() as jboolean)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_dev_oniris_audio_NativeEngine_nativeDecodedBytes(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) -> jlong {
    engine_mut(handle)
        .map(|engine| engine.decoded_bytes() as jlong)
        .unwrap_or(0)
}

fn engine_mut(handle: jlong) -> Option<&'static mut Engine> {
    if handle == 0 {
        None
    } else {
        // Safety: the Kotlin layer owns this pointer and serializes calls to the
        // engine except for the audio thread, which only calls nativeRender
        // while the rest of the engine is quiescent.
        Some(unsafe { &mut *(handle as *mut Engine) })
    }
}

fn fail(env: &mut JNIEnv, message: impl AsRef<str>, code: jint) -> jint {
    let _ = env.throw_new("java/lang/RuntimeException", message.as_ref());
    code
}

fn fail_string(env: &mut JNIEnv, message: impl AsRef<str>) -> jstring {
    let _ = env.throw_new("java/lang/RuntimeException", message.as_ref());
    std::ptr::null_mut()
}
