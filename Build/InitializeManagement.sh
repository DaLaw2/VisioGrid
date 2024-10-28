#!/bin/bash

CONFIG_PATH="./management.toml"
BACKUP_PATH="./default.toml"

if [ ! -w "$(dirname "$CONFIG_PATH")" ]; then
    echo "Error: No write permission to $(dirname "$CONFIG_PATH"). Please run this script as a user with appropriate permissions."
    exit 1
fi

if [ -f "$CONFIG_PATH" ]; then
    mv "$CONFIG_PATH" "$BACKUP_PATH"
    if [ $? -eq 0 ]; then
        echo "Moved $CONFIG_PATH to $BACKUP_PATH."
    else
        echo "Error: Unable to move $CONFIG_PATH to $BACKUP_PATH."
        exit 1
    fi
else
    echo "Warning: $CONFIG_PATH does not exist. A new configuration file will be created."
fi

declare -A config_defaults=(
    ["internal_timestamp"]="10"
    ["agent_listen_port"]="9090"
    ["http_server_bind_port"]="8080"
    ["dedicated_port_range"]="60000,65535"
    ["refresh_interval"]="5"
    ["polling_interval"]="50"
    ["bind_retry_duration"]="30"
    ["agent_idle_duration"]="5"
    ["control_channel_timeout"]="15"
    ["data_channel_timeout"]="15"
    ["file_transfer_timeout"]="15"
)

prompt_input() {
    local prompt_message="$1"
    local default_value="$2"
    local key="$3"
    local input
    while true; do
        read -p "$prompt_message [Default: $default_value]: " input
        input="${input:-$default_value}"
        if validate_input "$key" "$input"; then
            echo "$input"
            return
        else
            echo "Invalid input, please re-enter." >&2
        fi
    done
}

validate_input() {
    local key="$1"
    local value="$2"
    case "$key" in
        mode)
            if [[ "$value" == "frame" || "$value" == "time" ]]; then
                return 0
            fi
            ;;
        segment_duration_secs)
            if [[ "$value" =~ ^[0-9]+$ && "$value" -gt 0 ]]; then
                return 0
            fi
            ;;
        internal_timestamp|agent_listen_port|http_server_bind_port|refresh_interval|polling_interval|bind_retry_duration|agent_idle_duration|control_channel_timeout|data_channel_timeout|file_transfer_timeout)
            if [[ "$value" =~ ^[0-9]+$ ]]; then
                return 0
            fi
            ;;
        dedicated_port_range)
            if [[ "$value" =~ ^[0-9]+,[0-9]+$ ]]; then
                IFS=',' read -r start end <<< "$value"
                if [ "$start" -le "$end" ]; then
                    return 0
                fi
            fi
            ;;
        *)
            return 1
            ;;
    esac
    return 1
}

{
    echo "[Config]"

    mode_default="time"
    mode_input=$(prompt_input "Please enter video split mode (frame or time)" "$mode_default" "mode")

    if [ "$mode_input" == "time" ]; then
        segment_default="60"
        segment_duration=$(prompt_input "Please enter segment_duration_secs (seconds)" "$segment_default" "segment_duration_secs")
        echo "split_mode = { mode = \"$mode_input\", segment_duration_secs = $segment_duration }"
    else
        echo "split_mode = { mode = \"$mode_input\" }"
    fi

    for key in internal_timestamp agent_listen_port http_server_bind_port dedicated_port_range refresh_interval polling_interval bind_retry_duration agent_idle_duration control_channel_timeout data_channel_timeout file_transfer_timeout; do
        value=$(prompt_input "Please enter $key" "${config_defaults[$key]}" "$key")
        if [ "$key" == "dedicated_port_range" ]; then
            echo "$key = [$value]"
        elif [[ "$key" == *port* ]]; then
            echo "$key = $value"
        elif [[ "$key" == *interval* ]]; then
            if [[ "$key" == "polling_interval" ]]; then
                echo "$key = $value"
            else
                echo "$key = $value"
            fi
        else
            echo "$key = $value"
        fi
    done
} > "$CONFIG_PATH"

if [ $? -eq 0 ]; then
    echo "Initialization completed"
else
    echo "Error: Could not create $CONFIG_PATH."
    exit 1
fi

CONTAINER_IP=$(hostname -I | awk '{print $1}')
echo "Container IP is: $CONTAINER_IP"
