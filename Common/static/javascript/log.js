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
            document.getElementById('log-container').innerHTML = data;
        })
        .catch(error => {
            console.error('Fetch error:', error);
            document.getElementById('log-container').textContent = 'Error loading system log.';
        });
}

function loadAgentLog() {
    const agentId = document.getElementById('agent-id-input').value;
    if (!agentId) {
        alert("Please enter a Agent ID.");
        return;
    }
    lastLogType = agentId;
    lastUpdate = new Date();
    fetch(`/log/${agentId}`)
        .then(response => {
            if (!response.ok)
                throw new Error('Network response was not ok.');
            return response.text();
        })
        .then(data => {
            document.getElementById('log-container').innerHTML = data;
        })
        .catch(error => {
            console.error('Fetch error:', error);
            document.getElementById('log-container').textContent = `Error loading agent ${agentId} log.`;
        });
}

function updateLog() {
    const since = formatDate(lastUpdate);
    let updatePath;
    if (lastLogType === 'system')
        updatePath = `/log/system_log/since/${since}`;
    else
        updatePath = `/log/${lastLogType}/since/${since}`;
    fetch(updatePath)
        .then(response => {
            if (!response.ok)
                throw new Error('Network response was not ok.');
            return response.text();
        })
        .then(data => {
            if (data) {
                document.getElementById('log-container').innerHTML += data;
                lastUpdate = new Date();
            }
        })
        .catch(error => console.error('Fetch error:', error));
}

setInterval(updateLog, 10000);
