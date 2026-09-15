use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

const TEMPLATE: &str =
    "{prefix:<8} [{elapsed_precise}] {bar:47.#bcbddc/#756bb1} {percent:>3}% [ETA {eta}]";
const PROGRESS_CHARS: &str = "##-";

fn set_progressbar(pb: &ProgressBar) {
    pb.set_style(
        ProgressStyle::with_template(TEMPLATE)
            .expect("valid progress template")
            .progress_chars(PROGRESS_CHARS),
    );
}

pub fn new_multi_progressbar(
    multi: &MultiProgress,
    len: u64,
    prefix: impl Into<String>,
) -> ProgressBar {
    let pb = multi.add(ProgressBar::new(len));
    set_progressbar(&pb);
    pb.set_prefix(prefix.into());
    pb
}
