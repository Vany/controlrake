package obs

import (
	"github.com/Vany/controlrake/src/types"
	obsws "github.com/christopher-dG/go-obs-websocket"
	"log"
)

type Obs struct {
	c obsws.Client
}

func (o *Obs) Init() types.Connector {
	c := obsws.Client{Host: "localhost",
		Port:     4444,
		Password: "1234567890",
	}
	o.c = c
	return o
}

func (o *Obs) Stop() types.Connector {
	o.c.Disconnect()
	return o
}

func (o *Obs) connect() error {
	if err := o.c.Connect(); err != nil {
		log.Printf("OBS:error: %v", err)
		return err
	}
	o.c.AddEventHandler("Heartbeat", func(e obsws.Event) {
		log.Printf("E: %#v", e)
	})
	return nil
}

func (o *Obs) Handle(method string, arg interface{}) {
	if !o.c.Connected() {
		if o.connect() != nil {
			return
		}
	}

	req := obsws.NewSetHeartbeatRequest(true)
	if err := req.Send(o.c); err != nil {
		log.Fatal(err)
	}

	// This will block until the response comes (potentially forever).
	resp, err := req.Receive()
	if err != nil {
		log.Fatal(err)
	}
	log.Printf("%#v", resp)

}

func New() *Obs {
	return new(Obs)
}
