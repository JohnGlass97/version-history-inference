use indicatif::ProgressStyle;
use std::sync::LazyLock;

pub static PB_BAR_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template("[{elapsed_precise}] {prefix:20} {bar:60} {pos:>7}/{len:7} {msg}")
        .unwrap()
});

pub static PB_SPINNER_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template("[{elapsed_precise}] {prefix:20} {spinner} {msg}").unwrap()
});
