package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"
)

func StringPrompt(label string) string {
	var s string
	r := bufio.NewReader(os.Stdin)
	for {
		fmt.Fprint(os.Stderr, label)
		s, _ = r.ReadString('\n')
		if s != "" {
			break
		}
	}
	return strings.TrimSpace(s)
}

func main() {
	answer := StringPrompt("Question? ")
	fmt.Printf("Answer: %s\n", answer)
}
