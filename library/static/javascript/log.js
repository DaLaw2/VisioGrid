let lastLogType = 'system';
let lastUpdate = new Date();

function formatDate(date) {
    function pad(number) {
        if (number < 10)
            return '0' + number;
        return number;
    }
    return date.getFullYear() + '-' + pad(date.getMonth() + 1) + '-' + pad(date.getDate()) + '-' + pad(date.getHours()) + '-' + pad(date.getMinutes()) + '-' + pad(date.getSeconds());
}

function formatLogText(logText) {
    return logText.replace(/\n/g, '<br>');
}

function loadSystemLog() {
    lastLogType = 'system';
    lastUpdate = new Date();
    fetch('/log/system_log')
        .then(response => {
            if (!response.ok)
                throw new Error('Network response was not ok.');
            return response.text();
        })
        .then(data => {
            document.getElementById('log-container').innerHTML = formatLogText(data);
        })
        .catch(error => {
            console.error('Fetch error:', error);
            document.getElementById('log-container').textContent = 'Error loading system log.';
        });
}

function loadNodeLog() {
    const nodeId = document.getElementById('node-id-input').value;
    if (!nodeId) {
        alert("Please enter a Node ID.");
        return;
    }
    lastLogType = nodeId;
    lastUpdate = new Date();
    fetch(`/log/${nodeId}`)
        .then(response => {
            if (!response.ok)
                throw new Error('Network response was not ok.');
            return response.text();
        })
        .then(data => {
            document.getElementById('log-container').innerHTML = formatLogText(data);
        })
        .catch(error => {
            console.error('Fetch error:', error);
            document.getElementById('log-container').textContent = `Error loading node ${nodeId} log.`;
        });
}

function updateLog() {
    const since = formatDate(lastUpdate);
    let updatePath;
    if (lastLogType === 'system')
        updatePath = `/log/system_log/update/${since}`;
    else
        updatePath = `/log/${lastLogType}/update/${since}`;
    fetch(updatePath)
        .then(response => {
            if (!response.ok)
                throw new Error('Network response was not ok.');
            return response.text();
        })
        .then(data => {
            if (data) {
                document.getElementById('log-container').innerHTML += formatLogText(data);
                lastUpdate = new Date();
            }
        })
        .catch(error => console.error('Fetch error:', error));
}

setInterval(updateLog, 10000);
