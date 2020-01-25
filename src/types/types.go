package types

import "encoding/json"

type Connector interface {
	Handle(method string, arg interface{})
	Init() Connector
}

type WebMessage struct {
	Module string          `json:"module"`
	Method string          `json:"method"`
	Arg    json.RawMessage `json:"arg"`
}
