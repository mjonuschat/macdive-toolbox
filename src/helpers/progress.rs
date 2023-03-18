use indicatif::{ProgressBar, ProgressStyle};

pub(crate) fn create_spinner(msg: &str) -> anyhow::Result<ProgressBar> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[
                "▹▹▹▹▹",
                "▸▹▹▹▹",
                "▹▸▹▹▹",
                "▹▹▸▹▹",
                "▹▹▹▸▹",
                "▹▹▹▹▸",
                "▪▪▪▪▪",
            ])
            .template("{msg} {spinner:.blue}")?,
    );
    pb.set_message(msg.to_owned());
    Ok(pb)
}
