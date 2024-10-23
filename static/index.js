// ad astra per 𖫪

var WS = {};


function ConnectWebsocket(handler, callback) {
    let addr = "ws://" + location.host + "/" + handler;
    WS = new WebSocket(addr);
    console.log("Connecting: " + handler)
    WS.onmessage = onWSMessage;

    WS.onopen = () => {
        console.log("Connected to " + handler)
        callback()
        WS.onclose = () => {
            setTimeout(() => ConnectWebsocket(handler, callback), 1000);
        };
        WS.onerror = null;
    };

    WS.onerror = (ev) => {
        setTimeout(() => ConnectWebsocket(handler,callback), 1000);
    };
}

function EvaluateMyPath(obj, path) {
    if (!obj.classList.contains("widget")) {
        obj = obj.parentNode.closest(".widget");
    }

    let pa  = obj.parentNode.closest(".widget");
    if (pa) {
        return EvaluateMyPath(pa, path == undefined ? obj.id : obj.id + "|" + path)
    } else {
        return obj.id + "|" + path
    }
}

function Send(obj, msg) {
     if (WS.readyState == WebSocket.OPEN) {
        WS.send(EvaluateMyPath(obj) + "|" +msg)
    } else {
        console.log("lost: " + msg);
    }
}

