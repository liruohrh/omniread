(function() {
    let titleEl = $(".title_real");
    let title = text(titleEl).trim();
    
    let contentWrapper = $("#chapter-content");
    let id = attr(contentWrapper, "data-chapter-id");
    
    let paragraphs = [];
    
    let pElements = $$(".content_p_real");
    for (let i = 0; i < pElements.length; i++) {
        let textSpan = $(pElements[i], ".text_real");
        if (textSpan) {
            let t = text(textSpan).trim();
            if (t) {
                paragraphs.push({ type: "text", content: t });
            }
        }
    }
    
    let figures = $$(".illustration_wrapper_k2m8");
    for (let i = 0; i < figures.length; i++) {
        let fig = figures[i];
        let img = $(fig, ".illust_img_lazy");
        let caption = $(fig, ".illust_caption");
        
        let src = attr(img, "data-src") || attr(img, "data-original");
        let figNum = parseInt(attr(fig, "data-figure")) || 0;
        let insertIdx = Math.min(figNum * 3, paragraphs.length);
        
        paragraphs.splice(insertIdx, 0, {
            type: "image",
            src: src,
            alt: attr(img, "alt"),
            caption: caption ? text(caption).trim() : null
        });
    }
    
    let prevLink = $(".nav_prev:not(.disabled)");
    let nextLink = $(".nav_next:not(.disabled)");
    
    return {
        id: id,
        title: title,
        paragraphs: paragraphs,
        prev_url: prevLink ? attr(prevLink, "href") : null,
        next_url: nextLink ? attr(nextLink, "href") : null
    };
})();
