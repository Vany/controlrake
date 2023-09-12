// ad astra per 𖫪

//alert(10);


// var socket = new WebSocket("ws://localhost:8080/sw");

function draw(widgets) {
    document.getElementById("content").innerHTML = widgets
}


fetch("/widgets/")
    .then((response) => response.text())
    .then((text) => draw(text))

