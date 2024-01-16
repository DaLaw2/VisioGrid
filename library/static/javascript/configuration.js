async function loadCurrentConfig() {
    try {
        let response = await fetch('/configuration/get_config');
        if(response.ok) {
            let configuration = await response.json();
            document.getElementById('internal_timestamp').value = configuration.internal_timestamp;
            document.getElementById('node_listen_port').value = configuration.node_listen_port;
            document.getElementById('http_server_bind_port').value = configuration.http_server_bind_port;
            document.getElementById('bind_retry_duration').value = configuration.bind_retry_duration;
            document.getElementById('node_idle_duration').value = configuration.node_idle_duration;
            document.getElementById('polling_interval').value = configuration.polling_interval;
            document.getElementById('control_channel_timeout').value = configuration.control_channel_timeout;
            document.getElementById('data_channel_timeout').value = configuration.data_channel_timeout;
            document.getElementById('file_transfer_timeout').value = configuration.file_transfer_timeout;
            document.getElementById('dedicated_port_range_start').value = configuration.dedicated_port_range[0];
            document.getElementById('dedicated_port_range_end').value = configuration.dedicated_port_range[1];
            document.getElementById('font_path').value = configuration.font_path;
            document.getElementById('border_width').value = configuration.border_width;
            document.getElementById('font_size').value = configuration.font_size;
            document.getElementById('border_color_r').value = configuration.border_color[0];
            document.getElementById('border_color_g').value = configuration.border_color[1];
            document.getElementById('border_color_b').value = configuration.border_color[2];
            document.getElementById('text_color_r').value = configuration.text_color[0];
            document.getElementById('text_color_g').value = configuration.text_color[1];
            document.getElementById('text_color_b').value = configuration.text_color[2];
        } else {
            console.error('Failed to fetch current configuration:', response.statusText);
        }
    } catch (error) {
        console.error('Error:', error);
    }
}

async function submitForm() {
    const getRGBA = (idPrefix) => {
        return [
            parseInt(document.getElementById(idPrefix + '_r').value),
            parseInt(document.getElementById(idPrefix + '_g').value),
            parseInt(document.getElementById(idPrefix + '_b').value),
        ];
    };

    let configuration = {
        internal_timestamp: parseInt(document.getElementById('internal_timestamp').value),
        node_listen_port: parseInt(document.getElementById('node_listen_port').value),
        http_server_bind_port: parseInt(document.getElementById('http_server_bind_port').value),
        bind_retry_duration: parseInt(document.getElementById('bind_retry_duration').value),
        node_idle_duration: parseInt(document.getElementById('node_idle_duration').value),
        polling_interval: parseInt(document.getElementById('polling_interval').value),
        control_channel_timeout: parseInt(document.getElementById('control_channel_timeout').value),
        data_channel_timeout: parseInt(document.getElementById('data_channel_timeout').value),
        file_transfer_timeout: parseInt(document.getElementById('file_transfer_timeout').value),
        dedicated_port_range: [parseInt(document.getElementById('dedicated_port_range_start').value), parseInt(document.getElementById('dedicated_port_range_end').value)],
        font_path: document.getElementById('font_path').value,
        border_width: parseInt(document.getElementById('border_width').value),
        font_size: parseFloat(document.getElementById('font_size').value),
        border_color: getRGBA('border_color'),
        text_color: getRGBA('text_color')
    };

    try {
        let response = await fetch('/configuration/update_config', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(configuration)
        });
        let data = await response.text();

        if (response.ok) {
            alert(data);
            await loadCurrentConfig();
        } else {
            console.error('Failed to update configuration:', data);
            alert('Failed to update configuration: ' + data);
        }
    } catch (error) {
        console.error('Error submitting form:', error);
        alert('Error: ' + error);
    }
}
