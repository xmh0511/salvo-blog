#include <iostream>
#include <string>
#include <sys/ipc.h>
#include <sys/shm.h>
#include <cstring>
int main(){
	auto addr = shmat(65543,0,0);
	std::cout<< addr<<std::endl;
	auto ptr = (char*)addr;
	std::cout<< ptr;
}