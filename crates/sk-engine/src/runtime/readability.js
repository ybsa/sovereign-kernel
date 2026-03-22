// simple readability extracted text representation
(() => {
    // Basic fallback to extract meaningful text from a page
    const elementsToHide = document.querySelectorAll('script, style, nav, footer, iframe, noscript');
    elementsToHide.forEach(el => el.style.display = 'none');
    
    let text = document.body.innerText || "";
    
    elementsToHide.forEach(el => el.style.display = ''); // restore
    
    // Clean up excessive newlines
    return text.replace(/\n\s*\n/g, '\n\n').trim();
})();
