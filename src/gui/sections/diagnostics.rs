use dioxus::prelude::*;

pub(crate) fn render() -> Element {
    let mut refresh_sequence = use_signal(|| 0_u64);
    let copy_status = use_signal(String::new);
    let _ = refresh_sequence();
    let snapshot = crate::diagnostics_snapshot();
    let report = snapshot.report.clone();

    rsx! {
        section { class: "general-panel diagnostics-panel", id: "diagnostics-settings",
            div { class: "general-header section-head-row",
                div { class: "startup-copy",
                    h1 { "Diagnostics" }
                    p { "A local status summary for troubleshooting MuteGuard." }
                }
                button {
                    r#type: "button",
                    class: "secondary",
                    onclick: move |_| refresh_sequence += 1,
                    span { class: "solar-icon button-icon icon-diagnostics" }
                    "Refresh"
                }
            }

            div { class: "settings-card-grid",
                for section in snapshot.sections {
                    section { class: "sound-card diagnostics-card",
                        h2 { "{section.title}" }
                        dl { class: "diagnostics-grid",
                            for entry in section.entries {
                                div { class: "diagnostics-row",
                                    dt { "{entry.label}" }
                                    dd { "{entry.value}" }
                                }
                            }
                        }
                    }
                }

                section { class: "sound-card diagnostics-copy-card",
                    div { class: "sound-card-title",
                        div { class: "startup-copy",
                            h2 { "Support report" }
                            p { "Copies the values above without credentials or personal file paths." }
                        }
                    }
                    div { class: "diagnostics-actions",
                        button {
                            r#type: "button",
                            class: "secondary",
                            onclick: move |_| copy_diagnostics(report.clone(), copy_status),
                            span { class: "solar-icon button-icon icon-diagnostics" }
                            "Copy diagnostics"
                        }
                        if !copy_status().is_empty() {
                            span { class: "diagnostics-copy-status", role: "status", "{copy_status}" }
                        }
                    }
                }
            }
        }
    }
}

fn copy_diagnostics(report: String, mut status: Signal<String>) {
    spawn(async move {
        let report_literal = serde_json::to_string(&report).unwrap_or_else(|_| "\"\"".to_string());
        let script = format!(
            r#"
const text = {report_literal};
const area = document.createElement('textarea');
area.value = text;
area.setAttribute('readonly', '');
area.style.position = 'fixed';
area.style.opacity = '0';
document.body.appendChild(area);
area.select();
const copied = document.execCommand('copy');
area.remove();
return copied;
"#
        );
        let copied = dioxus::document::eval(&script)
            .await
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        status.set(if copied {
            "Copied".to_string()
        } else {
            "Copy failed".to_string()
        });
    });
}
