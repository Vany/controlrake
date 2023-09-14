// ad astra per 𖫪

(new Promise(function(resolve, reject) {
    var server = new WebSocket("ws://localhost/ws");
    server.onopen = () => resolve(server);
    server.onerror = reject;
    server.onmessage = onWSMessage;
}))
    .then((server) => WS = server)
    .catch(console.log)
;



fetch("/widgets/")
    .then((response) => response.text())
    .then((text) => {
        document.getElementById("content").innerHTML = text;
        for (let w of document.getElementsByClassName("widget")) {
            for (let s of w.getElementsByTagName("script")) {
                eval(s.innerText);
            }
        }
    })
;


//var WS = new WebSocket
function Send(obj, msg) {
    let w = obj.closest(".widget")
    let name = w.id
    WS.send(name + "|" +  msg)
    console.log(name, msg)
}

function onWSMessage(ev) {
    const [name, data] = ev.data.split("|", 2);
    let w = document.getElementById(name);
    let f = w.onWSEvent;
    if (f != null) f(data)
    else console.log(name, " have no ws event listener");
}




