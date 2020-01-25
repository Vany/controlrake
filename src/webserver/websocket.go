package webserver

import (
	"encoding/json"
	"github.com/Vany/controlrake/src/connector"
	"github.com/Vany/controlrake/src/types"
	"github.com/gorilla/websocket"
	"log"
	"net/http"
)

type WebSocket struct {
}

var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
}

func (ws *WebSocket) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Println(err)
		return
	}

	go func() {
		for {
			mt, msg, err := conn.ReadMessage()
			if err != nil {
				log.Printf("WS: %d, %s, %v", mt, msg, err)
				continue
			}
			wm := types.WebMessage{}
			if err := json.Unmarshal(msg, &wm); err != nil {
				log.Printf("WS Parse Error: %s, %v", msg, err)
				continue
			}
			connector.Handle(wm.Module, wm.Method, wm.Arg)
		}
	}()

}
