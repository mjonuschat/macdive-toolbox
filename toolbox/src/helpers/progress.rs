use indicatif::{ProgressState, ProgressStyle};
use tracing::{Span, info_span};
use tracing_indicatif::span_ext::IndicatifSpanExt;

pub(crate) fn elapsed_subsec(state: &ProgressState, writer: &mut dyn std::fmt::Write) {
    let seconds = state.elapsed().as_secs();
    let sub_seconds = (state.elapsed().as_millis() % 1000) / 100;
    let _ = writer.write_str(&format!("{}.{}s", seconds, sub_seconds));
}

pub(crate) fn header(command: &str) -> Span {
    // Output header
    let header_span = info_span!("header");
    header_span.pb_set_style(
        &ProgressStyle::with_template(
            &"Working on tasks for command: `@@@`. {wide_msg} {elapsed_subsec}\n{wide_bar}"
                .replace("@@@", command),
        )
        .unwrap()
        .with_key("elapsed_subsec", elapsed_subsec)
        .progress_chars("---"),
    );
    header_span.pb_start();

    // Bit of a hack to show a full "-----" line underneath the header.
    header_span.pb_set_length(1);
    header_span.pb_set_position(1);

    header_span
}
