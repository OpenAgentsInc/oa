// Add a simple animation to the title
document.addEventListener('DOMContentLoaded', () => {
    const title = document.querySelector('h1');
    title.style.opacity = '0';
    title.style.transition = 'opacity 1s ease-in';
    
    setTimeout(() => {
        title.style.opacity = '1';
    }, 100);
});