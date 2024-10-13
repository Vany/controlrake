package api

import (
	"github.com/andreykaipov/goobs"
	"github.com/vany/pirog"
	"reflect"
)

type Config struct {
	Server   string
	Password string
}

var AllEvents = reflect.Type(nil)

type Obs interface {
	AvailabilityNotification() *pirog.SUBSCRIPTION[bool, struct{}] // is obs available
	EventStream() *pirog.SUBSCRIPTION[reflect.Type, any]           // event from obs
	Execute(f func(*goobs.Client) error) error                     // execute in server context
}
