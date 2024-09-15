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
        .then(response => {
            if (response.ok) {
                alert('File uploaded successfully!');
            } else {
                response.text().then(errorMessage => {
                    alert('Upload failed: ' + errorMessage);
                });
            }
        })
        .catch(error => {
            alert('An error occurred: ' + error.message);
        });
}
