package api

import (
	"github.com/vany/pirog"
	"golang.org/x/net/context"
	"time"
)

type Config struct {
	Timeout time.Duration `default:"30s"`
}

type Request = pirog.REQUESTTYPE[string, string]
type ObsBrowser interface {
	ToWeb(context.Context, string) *Request // make a req to web
}
