package config

import (
	"context"
	"fmt"
	"github.com/creasty/defaults"
	"github.com/spf13/viper"
	httpserver_api "github.com/vany/controlrake/src/httpserver/api"
	obs_api "github.com/vany/controlrake/src/obs/api"
	obsbrowser_api "github.com/vany/controlrake/src/obsbrowser/api"
	widget_api "github.com/vany/controlrake/src/widget/api"
)

type Config struct {
	HTTP       httpserver_api.Config
	Obs        obs_api.Config
	ObsBrowser obsbrowser_api.Config
	Widget     widget_api.Config
}

type ConfigComponent struct {
	Config
	ErrorStatus error
}

func New() *ConfigComponent {
	c := &ConfigComponent{}
	viper.SetConfigName("config")
	viper.SetConfigType("yaml")
	viper.AddConfigPath(".")
	if err := defaults.Set(&c.Config); err != nil {
		c.ErrorStatus = fmt.Errorf("can't set defaults: %v", err)
	} else if err := viper.ReadInConfig(); err != nil {
		c.ErrorStatus = fmt.Errorf("can't viper.ReadInConfig(): %v", err)
	} else if err := viper.Unmarshal(&c.Config); err != nil {
		c.ErrorStatus = fmt.Errorf("can't viper.Unmarshal(&cfg): %v", err)
	}
	return c
}

func (c *ConfigComponent) Init(ctx context.Context) error {
	if c.ErrorStatus != nil {
		return fmt.Errorf("config was not found, %w", c.ErrorStatus)
	}
	return nil
}
