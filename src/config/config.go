package config

import (
	"encoding/json"
	"log"
	"os"
)

type CfgType map[string]interface{}

var cfg = CfgType{}

func New() CfgType {
	return make(CfgType)
}

func (c CfgType) Read(file string) CfgType {
	f, err := os.Open(file)
	if err != nil {
		log.Fatalf("Reading config: %v", err)
	}

	if err := json.NewDecoder(f).Decode(&cfg); err != nil {
		log.Fatalf("Reading config: %v", err)
	}

	return c
}
