package config

import (
	"context"
	"fmt"
	"github.com/spf13/viper"
	"github.com/vany/controlrake/src/widget"
)

type Config struct {
	BindAddress string
	StaticRoot  string
	Widgets     []widget.Config
}

func ReadConfig(ctx context.Context) (*Config, error) {
	viper.SetConfigName("config")
	viper.SetConfigType("yaml")
	viper.AddConfigPath(".")
	if err := viper.ReadInConfig(); err != nil {
		return nil, fmt.Errorf("can't viper.ReadInConfig(): %w", err)
	}
	cfg := &Config{}
	if err := viper.Unmarshal(&cfg); err != nil {
		return nil, fmt.Errorf("can't viper.Unmarshal(&cfg): %w", err)
	}

	return cfg, nil
}
