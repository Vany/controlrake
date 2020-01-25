// LICENSE APACHE 2.0
// (C) Vany Serezhkin 2020

package main

import (
	"context"
	"fmt"
	"github.com/Vany/controlrake/src/config"
	"github.com/Vany/controlrake/src/webserver"
)

func main() {
	cfg := config.New().Read("config.json")
	ctx, cFunc := context.WithCancel(context.Background())
	ws := webserver.New(cfg).Run(ctx)

	s := ""
	fmt.Scanf("%s", &s)

	cFunc()
	ws.Stop()
}
