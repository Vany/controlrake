// LICENSE APACHE 2.0
// (C) Vany Serezhkin 2024

package main

import (
	"context"
	"github.com/vany/controlrake/src/app"
	. "github.com/vany/pirog"
	"os"
	"os/signal"
)

func main() {
	ctx, _ := signal.NotifyContext(context.Background(), os.Interrupt)

	a := app.New(ctx)
	if a == nil {
		os.Exit(0)
	}

	ExecuteOnAllFields(ctx, a, "Run")
	<-ctx.Done()
	ExecuteOnAllFields(ctx, a, "Stop")
}
