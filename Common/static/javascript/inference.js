function submitForm() {
    const originalFormData = new FormData(document.getElementById('uploadForm'));
    const filteredFormData = new FormData();
    const modelTypeValue = document.querySelector('input[name="modelType"]:checked').value;
    filteredFormData.append('modelType', modelTypeValue);
    for (let [key, value] of originalFormData.entries()) {
        if (value && value.name) {
            filteredFormData.append(key, value, value.name);
        }
    }
    fetch('/inference/save_file', {
        method: 'POST',
        body: filteredFormData
    })
        .then(response => response.json())
        .then(data => {
            if (data.success) {
                alert('File uploaded successfully!');
            } else {
                alert('Upload failed: ' + data.error);
            }
        })
        .catch(error => {
            alert('An error occurred: ' + error.message);
        });
}
