package config

import (
	"context"
	"fmt"
	"github.com/spf13/viper"
)

type Config struct {
	BindAddress string
	StaticRoot  string
	SoundRoot   string
	Obs         map[string]any
	Widget      map[string]any
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

func (c *Config) InitStage1(ctx context.Context) error {
	return nil
}
