
#define _GNU_SOURCE 1	// needed for struct in6_pktinfo

#include <netinet/in.h>
#include <sys/socket.h>
#include <netinet/ip.h>
#include <string.h>

// constants to be exported in rust
int udp_sas_IP_PKTINFO       = IP_PKTINFO;
int udp_sas_IPV6_RECVPKTINFO = IPV6_RECVPKTINFO;

ssize_t udp_sas_recv(int sock, void* buf, size_t buf_len, int flags,
		struct sockaddr* src, socklen_t src_len,
		struct sockaddr* dst, socklen_t dst_len)
{
	struct iovec iov = {
		.iov_base = buf,
		.iov_len  = buf_len
	};
	char control[256];
	memset(src, 0, src_len);
	memset(dst, 0, dst_len);
	struct msghdr msg = {
		.msg_name	= src,
		.msg_namelen	= src_len,
		.msg_iov	= &iov,
		.msg_iovlen	= 1,
		.msg_control	= control,
		.msg_controllen = sizeof(control),
		.msg_flags	= 0,
	};

	ssize_t nb = recvmsg(sock, &msg, flags);
	if (nb >= 0) {
		// parse the ancillary data
		struct cmsghdr *cmsg;
		for (cmsg = CMSG_FIRSTHDR(&msg); cmsg != 0; cmsg = CMSG_NXTHDR(&msg, cmsg)) {
			// IPv4 destination (IP_PKTINFO)
			// NOTE: may also be present for v4-mapped addresses in IPv6
			if (cmsg->cmsg_level == IPPROTO_IP
					&& cmsg->cmsg_type == IP_PKTINFO
					&& dst_len >= sizeof(struct sockaddr_in)) {
				struct in_pktinfo* info = (struct in_pktinfo*) CMSG_DATA(cmsg);
				struct sockaddr_in* sa  = (struct sockaddr_in*) dst;

				sa->sin_family = AF_INET;
				sa->sin_port   = 0;	// not provided by the posix api
				sa->sin_addr   = info->ipi_spec_dst;
			}
			// IPv6 destination (IPV6_RECVPKTINFO)
			else if (cmsg->cmsg_level == IPPROTO_IPV6
					&& cmsg->cmsg_type == IPV6_PKTINFO
					&& dst_len >= sizeof(struct sockaddr_in6)) {
				struct in6_pktinfo* info = (struct in6_pktinfo*) CMSG_DATA(cmsg);
				struct sockaddr_in6* sa  = (struct sockaddr_in6*) dst;

				sa->sin6_family = AF_INET6;
				sa->sin6_port   = 0;	// not provided by the posix api
				sa->sin6_addr  	= info->ipi6_addr;
				sa->sin6_flowinfo = 0;
				sa->sin6_scope_id = 0;
			}
		}

	}
	return nb;
}


ssize_t udp_sas_send(int sock, void* buf, size_t buf_len, int flags,
		struct sockaddr* src, socklen_t src_len,
		struct sockaddr* dst, socklen_t dst_len)
{
	struct iovec iov = {
		.iov_base = buf,
		.iov_len  = buf_len
	};
	char control[256];
	struct msghdr msg = {
		.msg_name	= dst,
		.msg_namelen	= dst_len,
		.msg_iov	= &iov,
		.msg_iovlen	= 1,
		.msg_control	= control,
		.msg_controllen = sizeof(control),
		.msg_flags	= 0,
	};

	// add ancillary data
	//
	struct sockaddr_in*  sa4 = (struct sockaddr_in*)  src;
	struct sockaddr_in6* sa6 = (struct sockaddr_in6*) src;
	struct cmsghdr* cmsg = CMSG_FIRSTHDR(&msg);
	// IPv4 src address
	if ((src_len >= sizeof(struct sockaddr_in)) && (sa4->sin_family == AF_INET))
	{
		cmsg->cmsg_level = IPPROTO_IP;
		cmsg->cmsg_type  = IP_PKTINFO;
		struct in_pktinfo* info = (struct in_pktinfo*) CMSG_DATA(cmsg);
		memset(info, 0, sizeof(*info));
		info->ipi_spec_dst = sa4->sin_addr;
		cmsg->cmsg_len     = CMSG_LEN(sizeof(*info));
	}
	// IPv6 src address
	else if ((src_len >= sizeof(struct sockaddr_in6)) && (sa6->sin6_family == AF_INET6))
	{
		cmsg->cmsg_level = IPPROTO_IPV6;
		cmsg->cmsg_type  = IPV6_PKTINFO;
		struct in6_pktinfo* info = (struct in6_pktinfo*) CMSG_DATA(cmsg);
		memset(info, 0, sizeof(*info));
		info->ipi6_addr  = sa6->sin6_addr;
		cmsg->cmsg_len   = CMSG_LEN(sizeof(*info));
	}
	// no info
	else {
		cmsg->cmsg_len = 0;
	}

	msg.msg_controllen = cmsg->cmsg_len;

	return sendmsg(sock, &msg, flags);
}
