#include "rtl8139.h"
#include "pci.h"
#include "io.h"
#include "isr.h"
#include "serial.h"

#define RTL_REG_MAC0     0x00
#define RTL_REG_MAR0     0x08
#define RTL_REG_TSD0     0x10
#define RTL_REG_TSAD0    0x20
#define RTL_REG_RBSTART  0x30
#define RTL_REG_COMMAND  0x37
#define RTL_REG_CAPR     0x38
#define RTL_REG_CBR      0x3A
#define RTL_REG_IMR      0x3C
#define RTL_REG_ISR      0x3E
#define RTL_REG_TCR      0x40
#define RTL_REG_RCR      0x44
#define RTL_REG_CONFIG1  0x52

#define RTL_CMD_EMPTY    0x01
#define RTL_CMD_TE       0x04
#define RTL_CMD_RE       0x08
#define RTL_CMD_RESET    0x10

#define RTL_INT_ROK      0x0001
#define RTL_INT_TOK      0x0004

#define RX_BUFFER_SIZE   (8192 + 16 + 2048)

static uint16_t io_base = 0;
static uint8_t  mac_address[6] = {0};
static int      card_active = 0;

static uint8_t  rx_buffer[RX_BUFFER_SIZE] __attribute__((aligned(4)));
static uint8_t  tx_buffers[4][2048] __attribute__((aligned(4)));
static uint8_t  tx_cur = 0;
static uint16_t rx_offset = 0;

static uint32_t stat_rx_pkts = 0;
static uint32_t stat_tx_pkts = 0;
static uint32_t stat_rx_bytes = 0;
static uint32_t stat_tx_bytes = 0;

static void rtl8139_irq_handler(registers_t *regs) {
    (void)regs;
    if (!card_active) {
        return;
    }

    uint16_t isr = inw(io_base + RTL_REG_ISR);
    outw(io_base + RTL_REG_ISR, isr); // Acknowledge interrupts

    if (isr & RTL_INT_TOK) {
        // Transmit completed
    }
    if (isr & RTL_INT_ROK) {
        // Receive completed, packets will be consumed via receive_packet
    }
}

int rtl8139_init(void) {
    pci_device_t dev;
    if (!pci_find_device(0x10EC, 0x8139, &dev)) {
        serial_puts("[Nyxara RTL8139] Device 10EC:8139 not found on PCI bus.\n");
        return -1;
    }

    pci_enable_bus_master(&dev);
    io_base = (uint16_t)(dev.bar0 & ~0x3);

    serial_puts("[Nyxara RTL8139] Found at I/O port ");
    serial_puthex16(io_base);
    serial_puts(", IRQ ");
    serial_putdec(dev.irq);
    serial_puts("\n");

    // Power on device
    outb(io_base + RTL_REG_CONFIG1, 0x00);

    // Software reset
    outb(io_base + RTL_REG_COMMAND, RTL_CMD_RESET);
    while ((inb(io_base + RTL_REG_COMMAND) & RTL_CMD_RESET) != 0);

    // Read MAC address
    for (int i = 0; i < 6; i++) {
        mac_address[i] = inb(io_base + RTL_REG_MAC0 + i);
    }

    serial_puts("[Nyxara RTL8139] MAC Address: ");
    for (int i = 0; i < 6; i++) {
        serial_puthex8(mac_address[i]);
        if (i < 5) serial_puts(":");
    }
    serial_puts("\n");

    // Init RX buffer
    memset(rx_buffer, 0, sizeof(rx_buffer));
    outl(io_base + RTL_REG_RBSTART, (uint32_t)rx_buffer);

    // Initialize CAPR (Current Address of Packet Read) to 0xFFF0
    outw(io_base + RTL_REG_CAPR, 0xFFF0);

    // Enable interrupts: ROK (0x01) and TOK (0x04)
    outw(io_base + RTL_REG_IMR, RTL_INT_ROK | RTL_INT_TOK);

    // RCR: Accept Broadcast, Multicast, Physical match + Wrap (not promiscuous)
    outl(io_base + RTL_REG_RCR, 0x0E | (1 << 7));

    // Enable Transmit & Receive
    outb(io_base + RTL_REG_COMMAND, RTL_CMD_RE | RTL_CMD_TE);

    // Register IRQ handler
    isr_register_handler(32 + dev.irq, rtl8139_irq_handler);

    card_active = 1;
    rx_offset = 0;
    tx_cur = 0;
    return 0;
}

int rtl8139_is_active(void) {
    return card_active;
}

int rtl8139_has_packet(void) {
    if (!card_active) {
        return 0;
    }
    return (inb(io_base + RTL_REG_COMMAND) & RTL_CMD_EMPTY) == 0;
}

int rtl8139_get_mac(uint8_t *mac_out) {
    if (!card_active) {
        return -1;
    }
    memcpy(mac_out, mac_address, 6);
    return 0;
}

int rtl8139_send_packet(const void *data, uint32_t len) {
    if (!card_active || len > 1792 || len == 0) {
        return -1;
    }

    uint32_t tx_len = len < 60 ? 60 : len;
    memset(tx_buffers[tx_cur], 0, tx_len);
    memcpy(tx_buffers[tx_cur], data, len);
    outl(io_base + RTL_REG_TSAD0 + (tx_cur * 4), (uint32_t)tx_buffers[tx_cur]);
    outl(io_base + RTL_REG_TSD0 + (tx_cur * 4), tx_len & 0x1FFF);

    tx_cur = (tx_cur + 1) % 4;
    stat_tx_pkts++;
    stat_tx_bytes += tx_len;
    return (int)len;
}

int rtl8139_receive_packet(void *buf, uint32_t max_len) {
    if (!card_active) {
        return -1;
    }

    if ((inb(io_base + RTL_REG_COMMAND) & RTL_CMD_EMPTY) != 0) {
        return 0;
    }

    uint16_t *packet_header = (uint16_t *)(rx_buffer + rx_offset);
    uint16_t status = packet_header[0];
    uint16_t len = packet_header[1];

    if (!(status & RTL_INT_ROK) || len < 4) {
        return 0;
    }

    uint32_t packet_len = len - 4; // Strip 4-byte CRC
    uint32_t copy_len = packet_len < max_len ? packet_len : max_len;
    memcpy(buf, rx_buffer + rx_offset + 4, copy_len);

    rx_offset = (uint16_t)((rx_offset + len + 4 + 3) & ~3);
    outw(io_base + RTL_REG_CAPR, rx_offset - 0x10);
    rx_offset %= 8192;

    stat_rx_pkts++;
    stat_rx_bytes += copy_len;
    return (int)copy_len;
}

void rtl8139_get_stats(uint32_t *rx_pkts, uint32_t *tx_pkts, uint32_t *rx_bytes, uint32_t *tx_bytes) {
    if (rx_pkts)  *rx_pkts = stat_rx_pkts;
    if (tx_pkts)  *tx_pkts = stat_tx_pkts;
    if (rx_bytes) *rx_bytes = stat_rx_bytes;
    if (tx_bytes) *tx_bytes = stat_tx_bytes;
}