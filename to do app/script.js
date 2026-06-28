document.addEventListener('DOMContentLoaded', () => {
    const taskInput = document.getElementById('taskInput');
    const addButton = document.getElementById('addButton');
    const taskList = document.getElementById('taskList');

    // Load tasks from localStorage
    loadTasks();

    addButton.addEventListener('click', addTask);
    taskInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            addTask();
        }
    });

    function addTask() {
        const taskText = taskInput.value.trim();
        if (taskText !== '') {
            const li = document.createElement('li');
            li.textContent = taskText;

            const deleteButton = document.createElement('button');
            deleteButton.textContent = 'Delete';
            deleteButton.addEventListener('click', () => {
                taskList.removeChild(li);
                saveTasks();
            });

            const completeButton = document.createElement('button');
            completeButton.textContent = 'Complete';
            completeButton.addEventListener('click', () => {
                li.classList.toggle('completed');
                saveTasks();
            });

            li.appendChild(deleteButton);
            li.appendChild(completeButton);
            taskList.appendChild(li);

            taskInput.value = '';
            saveTasks();
        }
    }

    function saveTasks() {
        const tasks = Array.from(taskList.children).map(li => {
            return {
                text: li.textContent,
                completed: li.classList.contains('completed')
            };
        });
        localStorage.setItem('tasks', JSON.stringify(tasks));
    }

    function loadTasks() {
        const tasks = JSON.parse(localStorage.getItem('tasks')) || [];
        tasks.forEach(task => {
            const li = document.createElement('li');
            li.textContent = task.text;
            if (task.completed) {
                li.classList.add('completed');
            }

            const deleteButton = document.createElement('button');
            deleteButton.textContent = 'Delete';
            deleteButton.addEventListener('click', () => {
                taskList.removeChild(li);
                saveTasks();
            });

            const completeButton = document.createElement('button');
            completeButton.textContent = 'Complete';
            completeButton.addEventListener('click', () => {
                li.classList.toggle('completed');
                saveTasks();
            });

            li.appendChild(deleteButton);
            li.appendChild(completeButton);
            taskList.appendChild(li);
        });
    }
});
