async function loadCurrentConfig() {
    try {
        let response = await fetch('/config/get');
        if(response.ok) {
            let config = await response.json();
            document.getElementById('internal_timestamp').value = config.internal_timestamp;
            document.getElementById('agent_listen_port').value = config.agent_listen_port;
            document.getElementById('http_server_bind_port').value = config.http_server_bind_port;
            document.getElementById('dedicated_port_range_start').value = config.dedicated_port_range[0];
            document.getElementById('dedicated_port_range_end').value = config.dedicated_port_range[1];
            document.getElementById('refresh_interval').value = config.refresh_interval;
            document.getElementById('polling_interval').value = config.polling_interval;
            document.getElementById('bind_retry_duration').value = config.bind_retry_duration;
            document.getElementById('agent_idle_duration').value = config.agent_idle_duration;
            document.getElementById('control_channel_timeout').value = config.control_channel_timeout;
            document.getElementById('data_channel_timeout').value = config.data_channel_timeout;
            document.getElementById('file_transfer_timeout').value = config.file_transfer_timeout;
            document.getElementById('font_path').value = config.font_path;
            document.getElementById('font_size').value = config.font_size;
            document.getElementById('border_width').value = config.border_width;
            document.getElementById('border_color_r').value = config.border_color[0];
            document.getElementById('border_color_g').value = config.border_color[1];
            document.getElementById('border_color_b').value = config.border_color[2];
            document.getElementById('text_color_r').value = config.text_color[0];
            document.getElementById('text_color_g').value = config.text_color[1];
            document.getElementById('text_color_b').value = config.text_color[2];
        } else {
            console.error('Failed to fetch current config:', response.statusText);
        }
    } catch (error) {
        console.error('Error:', error);
    }
}

async function submitForm() {
    const getRange = (idPrefix) => {
        return [
            parseInt(document.getElementById(idPrefix + '_start').value),
            parseInt(document.getElementById(idPrefix + '_end').value)
        ];
    }

    const getRGB = (idPrefix) => {
        return [
            parseInt(document.getElementById(idPrefix + '_r').value),
            parseInt(document.getElementById(idPrefix + '_g').value),
            parseInt(document.getElementById(idPrefix + '_b').value),
        ];
    };

    let config = {
        internal_timestamp: parseInt(document.getElementById('internal_timestamp').value),
        agent_listen_port: parseInt(document.getElementById('agent_listen_port').value),
        http_server_bind_port: parseInt(document.getElementById('http_server_bind_port').value),
        dedicated_port_range: getRange('dedicated_port_range'),
        refresh_interval: parseInt(document.getElementById('refresh_interval').value),
        polling_interval: parseInt(document.getElementById('polling_interval').value),
        bind_retry_duration: parseInt(document.getElementById('bind_retry_duration').value),
        agent_idle_duration: parseInt(document.getElementById('agent_idle_duration').value),
        control_channel_timeout: parseInt(document.getElementById('control_channel_timeout').value),
        data_channel_timeout: parseInt(document.getElementById('data_channel_timeout').value),
        file_transfer_timeout: parseInt(document.getElementById('file_transfer_timeout').value),
        font_path: document.getElementById('font_path').value,
        font_size: parseFloat(document.getElementById('font_size').value),
        border_width: parseInt(document.getElementById('border_width').value),
        border_color: getRGB('border_color'),
        text_color: getRGB('text_color')
    };

    try {
        let response = await fetch('/config/update', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(config)
        });
        let data = await response.text();

        if (response.ok) {
            alert('Config updated successfully.')
            await loadCurrentConfig();
        } else {
            alert('Failed to update config: ' + data);
        }
    } catch (error) {
        console.error('Error submitting form:', error);
        alert('Error: ' + error);
    }
}
