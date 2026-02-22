const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;

let currentData = null;
let filteredData = null;
let sortColumn = null;
let sortDirection = 'asc';

// DOM elements
const dropZone = document.getElementById('dropZone');
const selectFileBtn = document.getElementById('selectFileBtn');
const fileInfo = document.getElementById('fileInfo');
const controls = document.getElementById('controls');
const tableContainer = document.getElementById('tableContainer');
const errorMessage = document.getElementById('errorMessage');
const searchInput = document.getElementById('searchInput');
const exportCsvBtn = document.getElementById('exportCsvBtn');
const exportJsonBtn = document.getElementById('exportJsonBtn');

// File selection
selectFileBtn.addEventListener('click', async () => {
    try {
        const selected = await open({
            multiple: false,
            filters: [{
                name: 'Data Files',
                extensions: ['csv', 'json', 'jsonl']
            }]
        });

        if (selected) {
            await loadFile(selected);
        }
    } catch (error) {
        showError(`Failed to select file: ${error}`);
    }
});

// Drag and drop via Tauri's native drag-drop events
const { listen } = window.__TAURI__.event;

listen('tauri://drag-over', () => {
    dropZone.classList.add('dragover');
});

listen('tauri://drag-leave', () => {
    dropZone.classList.remove('dragover');
});

listen('tauri://drag-drop', async (event) => {
    dropZone.classList.remove('dragover');
    const paths = event.payload.paths;
    if (paths && paths.length > 0) {
        await loadFile(paths[0]);
    }
});

// Load and parse file
async function loadFile(filePath) {
    try {
        hideError();
        
        // Determine file type
        const ext = filePath.split('.').pop().toLowerCase();
        let data;
        
        if (ext === 'csv') {
            data = await invoke('parse_csv', { filePath });
        } else if (ext === 'json') {
            data = await invoke('parse_json', { filePath });
        } else if (ext === 'jsonl') {
            data = await invoke('parse_jsonl', { filePath });
        } else {
            throw new Error('Unsupported file type. Please use CSV, JSON, or JSONL files.');
        }
        
        currentData = data;
        filteredData = { ...data };
        displayData(data);
        
    } catch (error) {
        showError(error);
    }
}

// Display data in table
function displayData(data) {
    // Update file info
    document.getElementById('fileName').textContent = data.file_name;
    document.getElementById('fileType').textContent = data.file_type;
    document.getElementById('rowCount').textContent = data.row_count.toLocaleString();
    document.getElementById('columnCount').textContent = data.headers.length;
    
    // Show elements
    dropZone.classList.add('hidden');
    fileInfo.classList.remove('hidden');
    controls.classList.remove('hidden');
    tableContainer.classList.remove('hidden');
    
    // Render table
    renderTable(data);
}

// Render table
function renderTable(data) {
    const tableHead = document.getElementById('tableHead');
    const tableBody = document.getElementById('tableBody');
    
    // Clear existing content
    tableHead.innerHTML = '';
    tableBody.innerHTML = '';
    
    // Create header row
    const headerRow = document.createElement('tr');
    data.headers.forEach((header, index) => {
        const th = document.createElement('th');
        th.textContent = header;
        th.dataset.column = index;
        
        // Add sorting
        th.addEventListener('click', () => sortTable(index));
        
        // Show current sort state
        if (sortColumn === index) {
            th.classList.add(sortDirection === 'asc' ? 'sorted-asc' : 'sorted-desc');
        }
        
        headerRow.appendChild(th);
    });
    tableHead.appendChild(headerRow);
    
    // Create data rows
    data.rows.forEach(row => {
        const tr = document.createElement('tr');
        row.forEach(cell => {
            const td = document.createElement('td');
            td.textContent = cell;
            tr.appendChild(td);
        });
        tableBody.appendChild(tr);
    });
}

// Sort table
function sortTable(columnIndex) {
    if (sortColumn === columnIndex) {
        sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
        sortColumn = columnIndex;
        sortDirection = 'asc';
    }
    
    const rows = [...filteredData.rows];
    rows.sort((a, b) => {
        const aVal = a[columnIndex] || '';
        const bVal = b[columnIndex] || '';
        
        // Try to parse as numbers
        const aNum = parseFloat(aVal);
        const bNum = parseFloat(bVal);
        
        if (!isNaN(aNum) && !isNaN(bNum)) {
            return sortDirection === 'asc' ? aNum - bNum : bNum - aNum;
        }
        
        // String comparison
        return sortDirection === 'asc' 
            ? aVal.localeCompare(bVal)
            : bVal.localeCompare(aVal);
    });
    
    filteredData.rows = rows;
    renderTable(filteredData);
}

// Search functionality
searchInput.addEventListener('input', (e) => {
    const searchTerm = e.target.value.toLowerCase();
    
    if (!searchTerm) {
        filteredData = { ...currentData };
        renderTable(filteredData);
        return;
    }
    
    const filtered = currentData.rows.filter(row => {
        return row.some(cell => 
            cell.toLowerCase().includes(searchTerm)
        );
    });
    
    filteredData = {
        ...currentData,
        rows: filtered,
        row_count: filtered.length
    };
    
    document.getElementById('rowCount').textContent = filtered.length.toLocaleString();
    renderTable(filteredData);
});

// Export to CSV
exportCsvBtn.addEventListener('click', async () => {
    try {
        const filePath = await save({
            defaultPath: 'exported_data.csv',
            filters: [{
                name: 'CSV File',
                extensions: ['csv']
            }]
        });
        
        if (filePath) {
            const result = await invoke('export_csv', {
                filePath,
                headers: currentData.headers,
                rows: filteredData.rows
            });
            alert(result);
        }
    } catch (error) {
        showError(`Export failed: ${error}`);
    }
});

// Export to JSON
exportJsonBtn.addEventListener('click', async () => {
    try {
        const filePath = await save({
            defaultPath: 'exported_data.json',
            filters: [{
                name: 'JSON File',
                extensions: ['json']
            }]
        });
        
        if (filePath) {
            const result = await invoke('export_json', {
                filePath,
                headers: currentData.headers,
                rows: filteredData.rows
            });
            alert(result);
        }
    } catch (error) {
        showError(`Export failed: ${error}`);
    }
});

// Error handling
function showError(error) {
    errorMessage.textContent = error;
    errorMessage.classList.remove('hidden');
}

function hideError() {
    errorMessage.classList.add('hidden');
}
