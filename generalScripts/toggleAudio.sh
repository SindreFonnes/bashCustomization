#!/usr/bin/env bash
# Toggle the default output between the PRO X headset and Ryzen line out.
# Standalone Bash utility, invoked by gscript / the toggleAudio alias.
# Requires a running PipeWire/WirePlumber session. --status does not switch.
set -euo pipefail

# The shell framework exports these from this machine's ignored local settings.
headset=${bashc_audio_headset_sink:-}
lineout=${bashc_audio_lineout_sink:-}

case "${1:-}" in
    ''|--status) ;;
    -h|--help) printf 'Usage: %s [--status]\n' "${0##*/}"; exit 0 ;;
    *) printf 'Usage: %s [--status]\n' "${0##*/}" >&2; exit 2 ;;
esac
if (( $# > 1 )); then
    printf 'Expected at most one argument.\n' >&2
    exit 2
fi
for tool in wpctl pw-dump jq; do
    command -v "$tool" >/dev/null || {
        printf 'toggleAudio requires %s (Linux with PipeWire/WirePlumber and jq).\n' "$tool" >&2
        exit 1
    }
done

current_sink() {
    wpctl inspect @DEFAULT_AUDIO_SINK@ | awk -F '"' '/node.name =/ {print $2}'
}

show_sink() {
    case "$1" in
        "$headset") printf 'Active output: PRO X Wireless Gaming Headset\n' ;;
        "$lineout") printf 'Active output: Line out - Ryzen HD Audio Controller\n' ;;
        *) printf 'Active output: %s\n' "$1" ;;
    esac
}

current=$(current_sink)
if [[ -z "$current" ]]; then
    printf 'Could not identify the default audio output.\n' >&2
    exit 1
fi
if [[ "${1:-}" == --status ]]; then
    show_sink "$current"
    exit 0
fi

if [[ -z "$headset" || -z "$lineout" || "$headset" == "$lineout" ]]; then
    printf 'toggleAudio: export two distinct device names as bashc_audio_headset_sink and bashc_audio_lineout_sink in local/local_variables.sh.\n' >&2
    exit 1
fi

# Resolve both live IDs from one snapshot. Never switch with an incomplete pair.
sinks=$(pw-dump)
sink_id() {
    jq -r --arg name "$1" '
        first(.[] | select(.info.props["media.class"] == "Audio/Sink"
            and .info.props["node.name"] == $name) | .id) // empty
    ' <<< "$sinks"
}
headset_id=$(sink_id "$headset")
lineout_id=$(sink_id "$lineout")
for sink in "$headset" "$lineout"; do
    if [[ "$sink" == "$headset" && -z "$headset_id" ]] ||
        [[ "$sink" == "$lineout" && -z "$lineout_id" ]]; then
        printf 'toggleAudio: output unavailable (disconnected or disabled): %s; nothing changed.\n' "$sink" >&2
        exit 1
    fi
done

case "$current" in
    "$headset") target=$lineout; target_id=$lineout_id ;;
    "$lineout") target=$headset; target_id=$headset_id ;;
    *) printf 'toggleAudio: active output is outside the configured pair; nothing changed.\n' >&2; exit 1 ;;
esac

wpctl set-default "$target_id"
# Give WirePlumber a moment to apply the change before reporting success.
for (( attempt=0; attempt<20; attempt++ )); do
    current=$(current_sink)
    if [[ "$current" == "$target" ]]; then
        show_sink "$current"
        exit 0
    fi
    sleep 0.1
done
printf 'The requested output did not become the default.\n' >&2
show_sink "$current" >&2
exit 1
