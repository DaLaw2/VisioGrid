#!/bin/bash

CONFIG_PATH="./agent.toml"
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
    ["management_address"]="127.0.0.1"
    ["management_port"]="9090"
    ["refresh_interval"]="5"
    ["polling_interval"]="50"
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
        internal_timestamp|management_port|refresh_interval|polling_interval|control_channel_timeout|data_channel_timeout|file_transfer_timeout)
            if [[ "$value" =~ ^[0-9]+$ ]]; then
                return 0
            fi
            ;;
        management_address)
            if [[ "$value" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
                IFS='.' read -r -a octets <<< "$value"
                for octet in "${octets[@]}"; do
                    if (( octet < 0 || octet > 255 )); then
                        return 1
                    fi
                done
                return 0
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

    internal_timestamp_default="${config_defaults["internal_timestamp"]}"
    internal_timestamp=$(prompt_input "Please enter internal_timestamp (milliseconds)" "$internal_timestamp_default" "internal_timestamp")
    echo "internal_timestamp = $internal_timestamp"

    management_address_default="${config_defaults["management_address"]}"
    management_address=$(prompt_input "Please enter management_address (IP address)" "$management_address_default" "management_address")
    echo "management_address = \"$management_address\""

    management_port_default="${config_defaults["management_port"]}"
    management_port=$(prompt_input "Please enter management_port (port)" "$management_port_default" "management_port")
    echo "management_port = $management_port"

    refresh_interval_default="${config_defaults["refresh_interval"]}"
    refresh_interval=$(prompt_input "Please enter refresh_interval (seconds)" "$refresh_interval_default" "refresh_interval")
    echo "refresh_interval = $refresh_interval"

    polling_interval_default="${config_defaults["polling_interval"]}"
    polling_interval=$(prompt_input "Please enter polling_interval (milliseconds)" "$polling_interval_default" "polling_interval")
    echo "polling_interval = $polling_interval"

    control_channel_timeout_default="${config_defaults["control_channel_timeout"]}"
    control_channel_timeout=$(prompt_input "Please enter control_channel_timeout (seconds)" "$control_channel_timeout_default" "control_channel_timeout")
    echo "control_channel_timeout = $control_channel_timeout"

    data_channel_timeout_default="${config_defaults["data_channel_timeout"]}"
    data_channel_timeout=$(prompt_input "Please enter data_channel_timeout (seconds)" "$data_channel_timeout_default" "data_channel_timeout")
    echo "data_channel_timeout = $data_channel_timeout"

    file_transfer_timeout_default="${config_defaults["file_transfer_timeout"]}"
    file_transfer_timeout=$(prompt_input "Please enter file_transfer_timeout (seconds)" "$file_transfer_timeout_default" "file_transfer_timeout")
    echo "file_transfer_timeout = $file_transfer_timeout"

} > "$CONFIG_PATH"

if [ $? -eq 0 ]; then
    echo "Initialization completed"
else
    echo "Error: Could not create $CONFIG_PATH."
    exit 1
fi
