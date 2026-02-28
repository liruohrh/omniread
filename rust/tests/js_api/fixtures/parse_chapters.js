(function() {
    let chapterItems = $$(".chapter_item_k2m9");
    let chapters = [];
    
    for (let i = 0; i < chapterItems.length; i++) {
        let item = chapterItems[i];
        
        if (hasClass(item, "chapter_item_fake") || hasClass(item, "hide-text")) {
            continue;
        }
        
        let id = attr(item, "data-chapter-id");
        let order = parseInt(attr(item, "data-order")) || 0;
        
        let link = $(item, ".chapter_link_real");
        let url = attr(link, "href");
        
        let numSpan = $(item, ".chapter_num");
        let titleSpan = $(item, ".chapter_title");
        let dateSpan = $(item, ".chapter_date");
        
        chapters.push({
            id: id,
            order: order,
            title: text(numSpan).trim() + " " + text(titleSpan).trim(),
            url: url,
            date: text(dateSpan).trim()
        });
    }
    
    chapters.sort(function(a, b) { return a.order - b.order; });
    return chapters;
})();
