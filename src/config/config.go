package config

import (
	"context"
	"fmt"
	"github.com/creasty/defaults"
	"github.com/spf13/viper"
	httpserver_api "github.com/vany/controlrake/src/httpserver/api"
	obs_api "github.com/vany/controlrake/src/obs/api"
	widget_api "github.com/vany/controlrake/src/widget/api"
)

type Config struct {
	HTTP   httpserver_api.Config
	Obs    obs_api.Config
	Widget widget_api.Config
}

func New() *Config {
	return &Config{}
}

func (c *Config) Init(ctx context.Context) error {
	viper.SetConfigName("config")
	viper.SetConfigType("yaml")
	viper.AddConfigPath(".")
	if err := defaults.Set(c); err != nil {
		return fmt.Errorf("can't set defaults: %w", err)
	} else if err := viper.ReadInConfig(); err != nil {
		return fmt.Errorf("can't viper.ReadInConfig(): %w", err)
	} else if err := viper.Unmarshal(&c); err != nil {
		return fmt.Errorf("can't viper.Unmarshal(&cfg): %w", err)
	}
	return nil
}
