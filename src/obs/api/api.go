package api

import "github.com/andreykaipov/goobs"

type Config struct {
	Server   string
	Password string
}

type Obs interface {
	Cli() *goobs.Client // get raw client
}
