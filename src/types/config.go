package types

import (
	"context"
	"github.com/spf13/viper"
	. "github.com/vany/pirog"
)

type Config struct {
	BindAddress string
	StaticRoot  string
	Widgets     []Widget
}

type Widget struct {
	WebCode   string
	Component string
}

func ReadConfigToContext(ctx context.Context) context.Context {
	viper.SetConfigName("config")
	viper.SetConfigType("yaml")
	viper.AddConfigPath(".")
	MUST(viper.ReadInConfig())
	cfg := &Config{}
	viper.Unmarshal(&cfg)

	_, cont := FromContext(ctx)
	if cont == nil {
		ctx = WithValues(ctx, cfg)
	} else {
		cont.Cfg = cfg
	}

	return ctx
}
