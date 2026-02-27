(function() {
    // Parse title
    let titleContainer = $(".title_main_2k9m");
    let titleSpans = $$(titleContainer, "span");
    let title = "";
    for (let i = 0; i < titleSpans.length; i++) {
        let span = titleSpans[i];
        if (hasClass(span, "f8k2j") && !hasClass(span, "hide-text")) {
            title = text(span).trim();
            break;
        }
    }

    // Parse author
    let authorLink = $(".author_name_real");
    let authorSpan = $(authorLink, ".f8k2j");
    let author = text(authorSpan).trim();

    // Parse cover URL
    let coverImg = $(".cover_img_lazy");
    let coverUrl = attr(coverImg, "data-src") || attr(coverImg, "data-original");

    // Parse status
    let statusEl = $(".status_real");
    let status = text(statusEl).trim();

    // Parse genres
    let genreEls = $$(".tag_item_m2k9");
    let genres = [];
    for (let i = 0; i < genreEls.length; i++) {
        let el = genreEls[i];
        if (hasClass(el, "f8k2j") && !hasClass(el, "hide-text") && attr(el, "data-genre")) {
            genres.push(text(el).trim());
        }
    }

    // Parse description
    let descEl = $(".desc_content_real");
    let descSpans = $$(descEl, "span");
    let descText = text(descEl);
    for (let i = 0; i < descSpans.length; i++) {
        if (hasClass(descSpans[i], "hide-text")) {
            descText = descText.replace(text(descSpans[i]), "");
        }
    }

    return {
        title: title,
        author: author,
        cover_url: coverUrl,
        description: descText.trim(),
        status: status,
        genres: genres
    };
})();
