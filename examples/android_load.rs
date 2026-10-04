//! dlopen smoke check: KNF computation and optional ONNX model loading.
use std::ffi::{c_char, CStr};

fn check(ort_library: &str, model: &str) -> eyre::Result<()> {
    let samples: Vec<f32> = (0..16000)
        .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / 16000.0).sin() * 0.5)
        .collect();
    let features = pyannote_rs::compute_fbank(&samples)?;
    eyre::ensure!(features.nrows() > 0 && features.ncols() == 80);
    eyre::ensure!(features.iter().all(|v| v.is_finite()));
    println!(
        "KNF: {} frames, {} bins",
        features.nrows(),
        features.ncols()
    );
    if ort_library.is_empty() && model.is_empty() {
        return Ok(());
    }
    ort::init_from(ort_library)?.commit();
    let session = ort::session::Session::builder()?
        .with_intra_threads(1)
        .map_err(|error| eyre::eyre!("{error}"))?
        .with_inter_threads(1)
        .map_err(|error| eyre::eyre!("{error}"))?
        .with_intra_op_spinning(false)
        .map_err(|error| eyre::eyre!("{error}"))?
        .with_inter_op_spinning(false)
        .map_err(|error| eyre::eyre!("{error}"))?
        .commit_from_file(model)?;
    eyre::ensure!(!session.inputs().is_empty() && !session.outputs().is_empty());
    println!(
        "ORT: model loaded, {} inputs, {} outputs",
        session.inputs().len(),
        session.outputs().len()
    );
    drop(session);
    Ok(())
}

/// Called only with non-null, NUL-terminated UTF-8 paths by the test loader.
#[no_mangle]
pub unsafe extern "C" fn pyannote_android_load(
    ort_library: *const c_char,
    model: *const c_char,
) -> i32 {
    if ort_library.is_null() || model.is_null() {
        return 1;
    }
    match std::panic::catch_unwind(|| {
        check(
            unsafe { CStr::from_ptr(ort_library) }.to_str()?,
            unsafe { CStr::from_ptr(model) }.to_str()?,
        )
    }) {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => {
            eprintln!("Android load check: {error:?}");
            1
        }
        Err(_) => {
            eprintln!("Android load check panicked");
            2
        }
    }
}
