package tests

import (
	"github.com/stretchr/testify/assert"
	"testing"
)

type A struct {
	A int
}

type B struct {
	A struct {
		C string
	}
	B int
}

func Test_X(t *testing.T) {
	var c chan string
	assert.Nil(t, c)
	c = make(chan string)
	assert.NotNil(t, c)
	close(c)
	c <- "lalal"
	assert.Nil(t, c)
}
